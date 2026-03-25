// SPDX-License-Identifier: MPL-2.0

//! `rustc_driver` wrapper for per-crate lockdep artifact extraction.
//!
//! The standalone front-end runs Cargo with this binary set as
//! `RUSTC_WORKSPACE_WRAPPER`. For each crate that should be analyzed, this
//! wrapper enters `after_analysis`, collects MIR-based facts, and writes one
//! JSON artifact into the shared artifact directory.

#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;

use std::{
    env,
    ops::Deref,
    path::{Path, PathBuf},
    process::exit,
};

use rustc_driver::Compilation;
use rustc_interface::interface;
use rustc_session::{EarlyDiagCtxt, config::ErrorOutputType};

fn arg_value<'a, T: Deref<Target = str>>(
    args: &'a [T],
    find_arg: &str,
    pred: impl Fn(&str) -> bool,
) -> Option<&'a str> {
    let mut args = args.iter().map(Deref::deref);
    while let Some(arg) = args.next() {
        let mut arg = arg.splitn(2, '=');
        if arg.next() != Some(find_arg) {
            continue;
        }

        match arg.next().or_else(|| args.next()) {
            Some(value) if pred(value) => return Some(value),
            _ => {}
        }
    }

    None
}

struct DefaultCallbacks;

impl rustc_driver::Callbacks for DefaultCallbacks {}

struct LockdepCallbacks;

impl rustc_driver::Callbacks for LockdepCallbacks {
    #[expect(rustc::bad_opt_access)]
    fn config(&mut self, config: &mut interface::Config) {
        config.opts.unstable_opts.mir_opt_level = Some(0);
    }

    fn after_analysis<'tcx>(
        &mut self,
        _: &rustc_interface::interface::Compiler,
        tcx: rustc_middle::ty::TyCtxt<'tcx>,
    ) -> Compilation {
        let output_dir = env::var("LOCKDEP_ARTIFACT_DIR")
            .map(PathBuf::from)
            .expect("LOCKDEP_ARTIFACT_DIR should be set when lockdep is enabled");

        tcx.dcx().abort_if_errors();
        if let Err(error) = analysis::write_artifact_to_dir(tcx, &output_dir) {
            tcx.dcx()
                .err(format!("failed to write lockdep artifact: {error}"));
        }
        tcx.dcx().abort_if_errors();

        Compilation::Continue
    }
}

fn main() {
    let early_dcx = EarlyDiagCtxt::new(ErrorOutputType::default());
    rustc_driver::init_rustc_env_logger(&early_dcx);

    exit(rustc_driver::catch_with_exit_code(move || {
        let mut orig_args: Vec<String> = env::args().collect();
        let has_sysroot_arg = arg_value(&orig_args, "--sysroot", |_| true).is_some();

        if let Some(pos) = orig_args.iter().position(|arg| arg == "--rustc") {
            orig_args.remove(pos);
            orig_args[0] = "rustc".to_string();
            run_compiler(orig_args, has_sysroot_arg, false);
            return;
        }

        if orig_args
            .iter()
            .any(|argument| argument == "--version" || argument == "-V")
        {
            let version_info = rustc_tools_util::get_version_info!();
            println!("{version_info}");
            return;
        }

        let wrapper_mode =
            orig_args.get(1).map(Path::new).and_then(Path::file_stem) == Some("rustc".as_ref());
        if wrapper_mode {
            orig_args.remove(1);
        }

        let lockdep_enabled = wrapper_mode && should_enable_lockdep();
        run_compiler(orig_args, has_sysroot_arg, lockdep_enabled);
    }));
}

fn run_compiler(mut args: Vec<String>, has_sysroot_arg: bool, lockdep_enabled: bool) {
    if !has_sysroot_arg {
        if let Ok(sysroot) = env::var("SYSROOT") {
            args.extend(["--sysroot".into(), sysroot]);
        }
    }

    if lockdep_enabled {
        rustc_driver::run_compiler(&args, &mut LockdepCallbacks);
    } else {
        rustc_driver::run_compiler(&args, &mut DefaultCallbacks);
    }
}

/// Returns whether lockdep analysis should run for the current rustc invocation.
fn should_enable_lockdep() -> bool {
    if env::var_os("LOCKDEP_ARTIFACT_DIR").is_none() {
        return false;
    }

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").ok();
    let workspace_root = env::var("LOCKDEP_WORKSPACE_ROOT").ok();
    if let (Some(manifest_dir), Some(workspace_root)) = (manifest_dir, workspace_root) {
        if !Path::new(&manifest_dir).starts_with(workspace_root) {
            return false;
        }
    }

    let selected_packages = env::var("LOCKDEP_SELECTED_PACKAGES").unwrap_or_default();
    if selected_packages.is_empty() {
        return true;
    }

    let current_package = env::var("CARGO_PKG_NAME").unwrap_or_default();
    selected_packages
        .split(',')
        .filter(|package| !package.is_empty())
        .any(|package| package == current_package)
}
