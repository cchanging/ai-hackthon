// SPDX-License-Identifier: MPL-2.0

//! Shared analysis library for the standalone lockdep tool.
//!
//! This crate defines:
//!
//! - the serialized artifact model;
//! - MIR collection logic;
//! - artifact read/write helpers used by the front-end and rustc wrapper.

#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_abi;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

mod collect;
mod model;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use rustc_middle::ty::TyCtxt;

pub use collect::collect_artifact;
pub use model::{
    AnalysisArtifact, FunctionArtifact, LocalOriginKey, LockClassKey, LockEdgeArtifact,
    LockEventArtifact, LockInfoArtifact, LockRootKey, LockUsageBitsArtifact,
    LockUsageSiteArtifact, LockUsageStateArtifact, ProjectionKey, SourceLocation,
};

/// Collects one crate artifact and writes it to the shared artifact directory.
pub fn write_artifact_to_dir<'tcx>(tcx: TyCtxt<'tcx>, output_dir: &Path) -> io::Result<PathBuf> {
    fs::create_dir_all(output_dir)?;

    let artifact = collect_artifact(tcx);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = format!(
        "{}-{}-{}-{timestamp}.json",
        sanitize(&artifact.package_name),
        sanitize(&artifact.crate_name),
        process::id()
    );
    let path = output_dir.join(file_name);
    let json = serde_json::to_vec_pretty(&artifact).map_err(io::Error::other)?;
    fs::write(&path, json)?;

    Ok(path)
}

/// Reads one serialized crate artifact from disk.
pub fn read_artifact(path: &Path) -> io::Result<AnalysisArtifact> {
    let json = fs::read(path)?;
    serde_json::from_slice(&json).map_err(io::Error::other)
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
}
