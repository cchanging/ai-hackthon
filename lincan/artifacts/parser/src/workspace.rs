use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use cargo_metadata::{Metadata, MetadataCommand};
use walkdir::WalkDir;

use crate::error::AppError;
use crate::model::DependencyInfo;

#[derive(Debug, Clone)]
pub struct WorkspaceMember {
    pub name: String,
    pub manifest_path: PathBuf,
    pub crate_root: PathBuf,
    pub source_roots: Vec<PathBuf>,
    pub rust_files: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct WorkspaceSnapshot {
    pub root: PathBuf,
    pub members: Vec<WorkspaceMember>,
    pub dependencies: Vec<DependencyInfo>,
}

pub fn load(input: &Path) -> Result<WorkspaceSnapshot, AppError> {
    let manifest_path = resolve_manifest_path(input)?;
    let metadata = load_metadata(&manifest_path)?;
    build_snapshot(metadata)
}

fn resolve_manifest_path(input: &Path) -> Result<PathBuf, AppError> {
    let candidate = if input.is_file() {
        input.to_path_buf()
    } else {
        input.join("Cargo.toml")
    };
    if candidate.exists() {
        return Ok(candidate);
    }
    Err(AppError::InvalidInput(format!(
        "cannot find Cargo.toml from input `{}`",
        input.display()
    )))
}

fn load_metadata(manifest_path: &Path) -> Result<Metadata, AppError> {
    let mut command = MetadataCommand::new();
    command.manifest_path(manifest_path);
    command
        .exec()
        .map_err(|err| AppError::CargoMetadata(err.to_string()))
}

fn build_snapshot(metadata: Metadata) -> Result<WorkspaceSnapshot, AppError> {
    let workspace_root = metadata.workspace_root.clone().into_std_path_buf();
    let member_ids: HashSet<_> = metadata.workspace_members.iter().cloned().collect();
    let packages = metadata.packages;

    let workspace_member_names: HashSet<String> = packages
        .iter()
        .filter(|pkg| member_ids.contains(&pkg.id))
        .map(|pkg| pkg.name.to_string())
        .collect();

    let mut members = Vec::new();
    for package in packages.iter().filter(|pkg| member_ids.contains(&pkg.id)) {
        let manifest_path = package.manifest_path.clone().into_std_path_buf();
        let Some(crate_root) = manifest_path.parent().map(Path::to_path_buf) else {
            return Err(AppError::InvalidInput(format!(
                "invalid manifest path for package `{}`: {}",
                package.name,
                manifest_path.display()
            )));
        };

        let mut source_root_set = BTreeSet::<PathBuf>::new();
        for target in &package.targets {
            let src_path = target.src_path.clone().into_std_path_buf();
            if let Some(parent) = src_path.parent() {
                source_root_set.insert(parent.to_path_buf());
            }
        }
        if source_root_set.is_empty() {
            source_root_set.insert(crate_root.join("src"));
        }

        let mut rust_file_set = BTreeSet::<PathBuf>::new();
        for source_root in &source_root_set {
            for rust_file in collect_rust_files(source_root)? {
                rust_file_set.insert(rust_file);
            }
        }

        let mut source_roots = source_root_set.into_iter().collect::<Vec<_>>();
        source_roots.sort_by(|a, b| {
            b.components()
                .count()
                .cmp(&a.components().count())
                .then_with(|| a.cmp(b))
        });
        let rust_files = rust_file_set.into_iter().collect::<Vec<_>>();

        members.push(WorkspaceMember {
            name: package.name.to_string(),
            manifest_path,
            crate_root,
            source_roots,
            rust_files,
        });
    }
    members.sort_by(|a, b| a.name.cmp(&b.name));

    let mut dependency_set: BTreeSet<(String, String)> = BTreeSet::new();
    for package in packages.iter().filter(|pkg| member_ids.contains(&pkg.id)) {
        for dep in &package.dependencies {
            if workspace_member_names.contains(dep.name.as_str()) {
                continue;
            }
            if dep.path.is_some() {
                continue;
            }
            dependency_set.insert((dep.name.to_string(), dep.req.to_string()));
        }
    }

    let dependencies = dependency_set
        .into_iter()
        .map(|(name, version)| DependencyInfo { name, version })
        .collect();

    Ok(WorkspaceSnapshot {
        root: workspace_root,
        members,
        dependencies,
    })
}

fn collect_rust_files(src_dir: &Path) -> Result<Vec<PathBuf>, AppError> {
    if !src_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(src_dir) {
        let entry = entry.map_err(|err| AppError::Extract(err.to_string()))?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}
