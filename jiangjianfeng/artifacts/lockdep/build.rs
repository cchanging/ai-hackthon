// SPDX-License-Identifier: MPL-2.0

fn main() {
    println!(
        "cargo:rustc-env=PROFILE={}",
        std::env::var("PROFILE").unwrap()
    );
    println!("cargo:rerun-if-changed=build.rs");
    rustc_tools_util::setup_version_info!();
}
