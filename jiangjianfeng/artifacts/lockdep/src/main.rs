// SPDX-License-Identifier: MPL-2.0

//! Front-end entry for the standalone static lockdep analyzer.
//!
//! This binary is responsible for:
//!
//! - parsing user-facing CLI arguments;
//! - invoking Cargo with the lockdep rustc wrapper;
//! - collecting per-crate analysis artifacts;
//! - aggregating those artifacts into global graph reports;
//! - rendering JSON, DOT, and terminal summaries.

#![feature(rustc_private)]

extern crate rustc_driver;

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    env,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::{self, Command},
    time::{SystemTime, UNIX_EPOCH},
};

use analysis::{
    AnalysisArtifact, LockClassKey, LockInfoArtifact, LockUsageBitsArtifact, LockUsageSiteArtifact,
    LockUsageStateArtifact, SourceLocation,
};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "Asterinas static lock dependency analysis driver"
)]
struct Cli {
    #[arg(long, default_value = "Cargo.toml")]
    manifest_path: PathBuf,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(short = 'p', long = "package")]
    packages: Vec<String>,
    #[arg(long)]
    target: Option<String>,
    #[arg(long)]
    features: Vec<String>,
    #[arg(long, default_value_t = false)]
    no_default_features: bool,
    #[arg(long)]
    emit_json: Option<PathBuf>,
    #[arg(long)]
    emit_dot: Option<PathBuf>,
    #[arg(long, default_value_t = false)]
    keep_artifacts: bool,
    #[arg(last = true)]
    cargo_args: Vec<String>,
}

/// A serialized summary of one complete lockdep analysis session.
#[derive(Debug, serde::Serialize)]
struct SessionSummary {
    workspace_root: String,
    artifact_dir: String,
    config: ConfigReport,
    crates: Vec<AnalysisArtifact>,
    global_report: GlobalReport,
}

/// The in-memory global lock graph built from aggregated crate artifacts.
#[derive(Debug, Clone)]
struct GlobalGraph {
    nodes: Vec<LockInfoArtifact>,
    edges: Vec<GraphEdge>,
    report: GlobalReport,
}

/// The top-level global report emitted to terminal and JSON.
#[derive(Debug, Clone, serde::Serialize)]
struct GlobalReport {
    lock_count: usize,
    edge_count: usize,
    cycle_count: usize,
    cycles: Vec<CycleReport>,
    atomic_mode_violation_count: usize,
    atomic_mode_violations: Vec<AtomicModeViolationReport>,
    single_lock_irq_violation_count: usize,
    single_lock_irq_violations: Vec<SingleLockIrqViolationReport>,
    irq_dependency_violation_count: usize,
    irq_dependency_violations: Vec<IrqDependencyViolationReport>,
    irq_conflict_count: usize,
    irq_conflicts: Vec<IrqConflictReport>,
    aa_deadlock_count: usize,
    aa_deadlocks: Vec<AaDeadlockReport>,
}

/// Origin metadata attached to a global graph edge.
#[derive(Debug, Clone, serde::Serialize)]
struct EdgeOrigin {
    crate_name: String,
    function: String,
    context: String,
    location: Option<SourceLocation>,
}

/// A concrete edge in the aggregated lock graph.
#[derive(Debug, Clone)]
struct GraphEdge {
    from: usize,
    to: usize,
    origin: EdgeOrigin,
}

/// One witness step in a reported lock cycle.
#[derive(Debug, Clone, serde::Serialize)]
struct CycleStep {
    from: LockInfoArtifact,
    to: LockInfoArtifact,
    origin: EdgeOrigin,
}

/// A reported global lock cycle with one witness path.
#[derive(Debug, Clone, serde::Serialize)]
struct CycleReport {
    locks: Vec<LockInfoArtifact>,
    steps: Vec<CycleStep>,
}

/// One violation where a sleeping lock is acquired while atomic-mode locks are held.
#[derive(Debug, Clone, serde::Serialize)]
struct AtomicModeViolationReport {
    kind: String,
    crate_name: String,
    function: String,
    context: String,
    held_lock: LockInfoArtifact,
    sleeping_lock: LockInfoArtifact,
    location: Option<SourceLocation>,
}

/// One concrete site where a lock is acquired in an IRQ-related context.
#[derive(Debug, Clone, serde::Serialize)]
struct IrqUseSite {
    crate_name: String,
    function: String,
    context: String,
    acquire: String,
    location: Option<SourceLocation>,
}

/// A minimal IRQ safety conflict report.
#[derive(Debug, Clone, serde::Serialize)]
struct IrqConflictReport {
    class: String,
    primitive: String,
    acquire: String,
    interrupt_sites: Vec<IrqUseSite>,
    interruptible_sites: Vec<IrqUseSite>,
}

/// One aggregated IRQ-related usage state for a lock mode across all crates.
#[derive(Debug, Clone)]
struct AggregatedLockUsageState {
    lock: LockInfoArtifact,
    bits: LockUsageBitsArtifact,
    first_hardirq_use: Option<IrqUseSite>,
    first_softirq_use: Option<IrqUseSite>,
    first_hardirq_enabled_use: Option<IrqUseSite>,
    first_hardirq_disabled_use: Option<IrqUseSite>,
    first_softirq_enabled_use: Option<IrqUseSite>,
    first_softirq_disabled_use: Option<IrqUseSite>,
}

/// One violation where a single lock mode is both IRQ-safe and IRQ-unsafe.
#[derive(Debug, Clone, serde::Serialize)]
struct SingleLockIrqViolationReport {
    kind: String,
    class: String,
    primitive: String,
    acquire: String,
    safe_site: Option<IrqUseSite>,
    unsafe_site: Option<IrqUseSite>,
}

/// One violation where a dependency edge leads from a safe lock to an unsafe lock.
#[derive(Debug, Clone, serde::Serialize)]
struct IrqDependencyViolationReport {
    kind: String,
    from: LockInfoArtifact,
    to: LockInfoArtifact,
    from_safe_site: Option<IrqUseSite>,
    to_unsafe_site: Option<IrqUseSite>,
    witness_edge_origin: EdgeOrigin,
}

/// One source site participating in an AA-style deadlock report.
#[derive(Debug, Clone, serde::Serialize)]
struct AaDeadlockSite {
    function: String,
    context: String,
    acquire: String,
    location: Option<SourceLocation>,
}

/// A report describing an AA/self-loop style deadlock risk.
#[derive(Debug, Clone, serde::Serialize)]
struct AaDeadlockReport {
    kind: String,
    class: String,
    primitive: String,
    sites: Vec<AaDeadlockSite>,
}

/// Temporary directories used during one analyzer invocation.
struct SessionDirs {
    root: PathBuf,
    artifacts: PathBuf,
    target: PathBuf,
}

/// Parsed `lockdep.toml` configuration.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct LockdepConfig {
    #[serde(default)]
    ignore: Vec<String>,
    #[serde(default)]
    irq_entries: Vec<IrqEntryConfig>,
    #[serde(default)]
    ordered_helpers: Vec<OrderedHelperConfig>,
}

/// A configured IRQ entry prototype.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct IrqEntryConfig {
    function: String,
    context: String,
    #[serde(default)]
    callback_arg_index: usize,
}

/// A configured ordered multi-lock helper prototype.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct OrderedHelperConfig {
    function: String,
    order: String,
}

/// A user-facing summary of loaded configuration.
#[derive(Debug, Clone, serde::Serialize)]
struct ConfigReport {
    path: Option<String>,
    ignore_count: usize,
    irq_entry_count: usize,
    ordered_helper_count: usize,
}

fn main() {
    let args = normalize_args(env::args_os());

    if let Err(error) = run(args) {
        eprintln!("cargo-lockdep: {error}");
        process::exit(1);
    }
}

fn normalize_args(args: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    let mut normalized = Vec::new();

    for (index, argument) in args.into_iter().enumerate() {
        if index == 1 && argument == "lockdep" {
            continue;
        }
        normalized.push(argument);
    }

    normalized
}

fn run(args: Vec<OsString>) -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse_from(args);
    let manifest_path = canonicalize_manifest_path(&cli.manifest_path)?;
    let workspace_root = manifest_path
        .parent()
        .ok_or_else(|| io::Error::other("manifest path has no parent directory"))?
        .to_path_buf();
    let (config, config_report) = load_lockdep_config(&workspace_root, cli.config.as_deref())?;
    let session_dirs = create_session_dirs()?;
    let driver_path = ensure_driver_exists()?;
    let sysroot = resolve_sysroot()?;

    let mut cargo = Command::new("cargo");
    cargo
        .arg("check")
        .arg("--manifest-path")
        .arg(&manifest_path)
        .env("RUSTC_WORKSPACE_WRAPPER", &driver_path)
        .env("LOCKDEP_ARTIFACT_DIR", &session_dirs.artifacts)
        .env("LOCKDEP_WORKSPACE_ROOT", &workspace_root)
        .env("LOCKDEP_SELECTED_PACKAGES", cli.packages.join(","))
        .env(
            "LOCKDEP_IRQ_ENTRIES_JSON",
            serde_json::to_string(&config.irq_entries)?,
        )
        .env("CARGO_TARGET_DIR", &session_dirs.target)
        .env("SYSROOT", sysroot);

    for package in &cli.packages {
        cargo.arg("--package").arg(package);
    }

    if let Some(target) = &cli.target {
        cargo.arg("--target").arg(target);
    }

    if cli.no_default_features {
        cargo.arg("--no-default-features");
    }

    for features in &cli.features {
        cargo.arg("--features").arg(features);
    }

    cargo.args(&cli.cargo_args);

    let status = cargo.status()?;
    if !status.success() {
        return Err(format!("cargo check failed with status {status}").into());
    }

    let (summary, global_graph) = load_summary(
        &workspace_root,
        &config,
        config_report,
        &session_dirs.artifacts,
    )?;
    print_summary(&summary);

    if let Some(path) = &cli.emit_json {
        let json = serde_json::to_vec_pretty(&summary)?;
        fs::write(path, json)?;
    }

    if let Some(path) = &cli.emit_dot {
        fs::write(path, render_dot(&global_graph))?;
    }

    if !cli.keep_artifacts {
        fs::remove_dir_all(&session_dirs.root)?;
    }

    Ok(())
}

/// Resolves the manifest path against the current directory when needed.
fn canonicalize_manifest_path(path: &Path) -> io::Result<PathBuf> {
    if path.is_absolute() {
        fs::canonicalize(path)
    } else {
        fs::canonicalize(env::current_dir()?.join(path))
    }
}

/// Creates per-run temporary artifact and target directories.
fn create_session_dirs() -> io::Result<SessionDirs> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let root = env::temp_dir().join(format!("cargo-lockdep-{timestamp}-{}", process::id()));
    let artifacts = root.join("artifacts");
    let target = root.join("target");
    fs::create_dir_all(&artifacts)?;
    fs::create_dir_all(&target)?;

    Ok(SessionDirs {
        root,
        artifacts,
        target,
    })
}

/// Ensures that the rustc wrapper binary exists and returns its path.
fn ensure_driver_exists() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let current_exe = env::current_exe()?;
    let driver_path = with_executable_name(current_exe, "lockdep-driver");

    let mut command = Command::new("cargo");
    command
        .arg("build")
        .arg("--manifest-path")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .arg("--bin")
        .arg("lockdep-driver");
    if should_build_driver_in_release_profile(&env::current_exe()?) {
        command.arg("--release");
    }
    let status = command.status()?;
    if !status.success() {
        return Err("failed to build lockdep-driver".into());
    }

    if driver_path.exists() {
        Ok(driver_path)
    } else {
        Err(format!("lockdep-driver not found at {}", driver_path.display()).into())
    }
}

fn with_executable_name(path: PathBuf, file_name: &str) -> PathBuf {
    let mut path = path.with_file_name(file_name);
    if cfg!(windows) {
        path.set_extension("exe");
    }
    path
}

fn should_build_driver_in_release_profile(current_exe: &Path) -> bool {
    current_exe
        .parent()
        .and_then(|parent| parent.file_name())
        .is_some_and(|name| name == "release")
}

fn resolve_sysroot() -> Result<String, Box<dyn std::error::Error>> {
    if let Ok(sysroot) = env::var("SYSROOT") {
        return Ok(sysroot);
    }

    let output = Command::new("rustc")
        .arg("--print")
        .arg("sysroot")
        .output()?;
    if !output.status.success() {
        return Err("failed to resolve rustc sysroot".into());
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::should_build_driver_in_release_profile;

    #[test]
    fn detects_release_driver_profile_from_exe_path() {
        assert!(should_build_driver_in_release_profile(Path::new(
            "/tmp/target/release/cargo-lockdep"
        )));
        assert!(!should_build_driver_in_release_profile(Path::new(
            "/tmp/target/debug/cargo-lockdep"
        )));
    }
}

/// Loads all crate artifacts and builds the global report.
fn load_summary(
    workspace_root: &Path,
    config: &LockdepConfig,
    config_report: ConfigReport,
    artifact_dir: &Path,
) -> Result<(SessionSummary, GlobalGraph), Box<dyn std::error::Error>> {
    let mut crates = fs::read_dir(artifact_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("json"))
        .map(|path| analysis::read_artifact(&path))
        .collect::<io::Result<Vec<_>>>()?;
    apply_ignore_filters(&mut crates, &config.ignore);
    crates.sort_by(|left, right| {
        left.package_name
            .cmp(&right.package_name)
            .then(left.crate_name.cmp(&right.crate_name))
    });

    let global_graph = build_global_graph(&crates);

    Ok((
        SessionSummary {
            workspace_root: workspace_root.display().to_string(),
            artifact_dir: artifact_dir.display().to_string(),
            config: config_report,
            crates,
            global_report: global_graph.report.clone(),
        },
        global_graph,
    ))
}

/// Prints the terminal summary for one completed analysis run.
fn print_summary(summary: &SessionSummary) {
    let crate_count = summary.crates.len();
    let function_count = summary
        .crates
        .iter()
        .map(|artifact| artifact.functions.len())
        .sum::<usize>();
    let lock_event_count = summary
        .crates
        .iter()
        .flat_map(|artifact| artifact.functions.iter())
        .map(|function| function.lock_events.len())
        .sum::<usize>();
    let lock_edge_count = summary
        .crates
        .iter()
        .flat_map(|artifact| artifact.functions.iter())
        .map(|function| function.lock_edges.len())
        .sum::<usize>();

    println!(
        "Analyzed {crate_count} crate(s), collected {function_count} MIR-backed function(s), {lock_event_count} lock event(s), and {lock_edge_count} lock edge(s)."
    );
    println!(
        "Detected {} potential lock cycle(s) in the global dependency graph.",
        summary.global_report.cycle_count
    );
    println!(
        "Detected {} atomic-mode violation(s).",
        summary.global_report.atomic_mode_violation_count
    );
    println!(
        "Detected {} single-lock IRQ safety violation(s) and {} IRQ dependency violation(s).",
        summary.global_report.single_lock_irq_violation_count,
        summary.global_report.irq_dependency_violation_count
    );
    println!(
        "Detected {} potential AA/self-loop deadlock(s).",
        summary.global_report.aa_deadlock_count
    );
    if let Some(path) = &summary.config.path {
        println!(
            "Loaded config from {} (ignore={}, irq_entries={}, ordered_helpers={}).",
            path,
            summary.config.ignore_count,
            summary.config.irq_entry_count,
            summary.config.ordered_helper_count,
        );
    }
    for artifact in &summary.crates {
        let lock_events = artifact
            .functions
            .iter()
            .map(|function| function.lock_events.len())
            .sum::<usize>();
        let lock_edges = artifact
            .functions
            .iter()
            .map(|function| function.lock_edges.len())
            .sum::<usize>();
        println!(
            "- {} ({}) : {} function(s), {} event(s), {} edge(s)",
            artifact.package_name,
            artifact.crate_name,
            artifact.functions.len(),
            lock_events,
            lock_edges,
        );
    }
    for (index, cycle) in summary.global_report.cycles.iter().take(3).enumerate() {
        println!("- cycle {}: {}", index + 1, format_cycle(cycle));
        for step in &cycle.steps {
            let location = step
                .origin
                .location
                .as_ref()
                .map(|location| format!("{}:{}:{}", location.file, location.line, location.column))
                .unwrap_or_else(|| "<unknown>".into());
            println!(
                "  {} -> {} via {} [{}] at {}",
                short_lock_label(&step.from),
                short_lock_label(&step.to),
                step.origin.function,
                step.origin.context,
                location,
            );
        }
    }
    for (index, violation) in summary
        .global_report
        .atomic_mode_violations
        .iter()
        .take(3)
        .enumerate()
    {
        println!(
            "- atomic mode violation {}: {} -> {}",
            index + 1,
            short_lock_label(&violation.held_lock),
            short_lock_label(&violation.sleeping_lock),
        );
        println!(
            "  witness: {} [{}] {}",
            violation.function,
            violation.context,
            format_site_location(violation.location.as_ref()),
        );
    }
    for (index, conflict) in summary
        .global_report
        .single_lock_irq_violations
        .iter()
        .take(3)
        .enumerate()
    {
        println!(
            "- single-lock irq violation {}: {} {}({})",
            index + 1,
            conflict.kind,
            conflict.primitive,
            conflict.class,
        );
        if let Some(site) = &conflict.safe_site {
            println!(
                "  safe site: {} [{} {}] {}",
                site.function,
                site.context,
                site.acquire,
                format_site_location(site.location.as_ref()),
            );
        }
        if let Some(site) = &conflict.unsafe_site {
            println!(
                "  unsafe site: {} [{} {}] {}",
                site.function,
                site.context,
                site.acquire,
                format_site_location(site.location.as_ref()),
            );
        }
    }
    for (index, violation) in summary
        .global_report
        .irq_dependency_violations
        .iter()
        .take(3)
        .enumerate()
    {
        println!(
            "- irq dependency violation {}: {} {} -> {}",
            index + 1,
            violation.kind,
            short_lock_label(&violation.from),
            short_lock_label(&violation.to),
        );
        println!(
            "  witness edge: {} [{}] {}",
            violation.witness_edge_origin.function,
            violation.witness_edge_origin.context,
            format_site_location(violation.witness_edge_origin.location.as_ref()),
        );
    }
    for (index, conflict) in summary
        .global_report
        .irq_conflicts
        .iter()
        .take(3)
        .enumerate()
    {
        println!(
            "- irq conflict {}: {} seen in interrupt and interruptible contexts",
            index + 1,
            format!("{}({})", conflict.primitive, conflict.class),
        );
        if let Some(site) = conflict.interrupt_sites.first() {
            println!(
                "  interrupt site: {} [{} {}] {}",
                site.function,
                site.context,
                site.acquire,
                format_site_location(site.location.as_ref()),
            );
        }
        if let Some(site) = conflict.interruptible_sites.first() {
            println!(
                "  interruptible site: {} [{} {}] {}",
                site.function,
                site.context,
                site.acquire,
                format_site_location(site.location.as_ref()),
            );
        }
    }
    for (index, aa) in summary
        .global_report
        .aa_deadlocks
        .iter()
        .take(3)
        .enumerate()
    {
        println!(
            "- aa deadlock {}: {} {}",
            index + 1,
            aa.kind,
            format!("{}({})", aa.primitive, aa.class),
        );
        for site in &aa.sites {
            println!(
                "  {} [{} {}] {}",
                site.function,
                site.context,
                site.acquire,
                format_site_location(site.location.as_ref()),
            );
        }
    }
}

/// Applies `ignore` prefix filters from configuration to function summaries.
fn apply_ignore_filters(crates: &mut [AnalysisArtifact], ignore_prefixes: &[String]) {
    if ignore_prefixes.is_empty() {
        return;
    }

    for crate_artifact in crates {
        crate_artifact.functions.retain(|function| {
            !ignore_prefixes
                .iter()
                .any(|prefix| function.def_path.starts_with(prefix))
        });
    }
}

/// Loads the optional `lockdep.toml` file and prepares a user-facing config report.
fn load_lockdep_config(
    workspace_root: &Path,
    config_path: Option<&Path>,
) -> Result<(LockdepConfig, ConfigReport), Box<dyn std::error::Error>> {
    let path = config_path
        .map(|path| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                workspace_root.join(path)
            }
        })
        .unwrap_or_else(|| workspace_root.join("lockdep.toml"));
    if !path.exists() {
        return Ok((
            LockdepConfig::default(),
            ConfigReport {
                path: None,
                ignore_count: 0,
                irq_entry_count: 0,
                ordered_helper_count: 0,
            },
        ));
    }

    let content = fs::read_to_string(&path)?;
    let config: LockdepConfig = toml::from_str(&content)?;
    let report = ConfigReport {
        path: Some(path.display().to_string()),
        ignore_count: config.ignore.len(),
        irq_entry_count: config.irq_entries.len(),
        ordered_helper_count: config.ordered_helpers.len(),
    };
    Ok((config, report))
}

/// Builds the global lock graph and all top-level reports from crate artifacts.
fn build_global_graph(crates: &[AnalysisArtifact]) -> GlobalGraph {
    let mut node_ids = BTreeMap::<LockInfoArtifact, usize>::new();
    let mut reverse_nodes = Vec::<LockInfoArtifact>::new();
    let mut adjacency = BTreeMap::<usize, BTreeSet<usize>>::new();
    let mut edges = Vec::<GraphEdge>::new();

    for crate_artifact in crates {
        for function in &crate_artifact.functions {
            for edge in &function.lock_edges {
                if !is_deadlock_relevant_edge(&edge.from, &edge.to) {
                    continue;
                }
                let from = intern_lock(&mut node_ids, &mut reverse_nodes, &edge.from);
                let to = intern_lock(&mut node_ids, &mut reverse_nodes, &edge.to);
                let is_new = adjacency.entry(from).or_default().insert(to);
                if is_new {
                    edges.push(GraphEdge {
                        from,
                        to,
                        origin: EdgeOrigin {
                            crate_name: crate_artifact.package_name.clone(),
                            function: function.def_path.clone(),
                            context: edge.context.clone(),
                            location: edge.location.clone(),
                        },
                    });
                }
            }
        }
    }

    let cycles = find_cycles(&reverse_nodes, &adjacency, &edges);
    let atomic_mode_violations = find_atomic_mode_violations(crates);
    let usage_states = build_global_lock_usage_states(crates);
    let single_lock_irq_violations = find_single_lock_irq_violations(&usage_states);
    let irq_dependency_violations =
        find_irq_dependency_violations(&edges, &reverse_nodes, &usage_states);
    let irq_conflicts =
        legacy_irq_conflicts_from_single_lock_violations(&single_lock_irq_violations);
    let aa_deadlocks = find_aa_deadlocks(&reverse_nodes, &edges, &irq_conflicts);
    let edge_count = adjacency.values().map(BTreeSet::len).sum();

    let report = GlobalReport {
        lock_count: reverse_nodes.len(),
        edge_count,
        cycle_count: cycles.len(),
        cycles,
        atomic_mode_violation_count: atomic_mode_violations.len(),
        atomic_mode_violations,
        single_lock_irq_violation_count: single_lock_irq_violations.len(),
        single_lock_irq_violations,
        irq_dependency_violation_count: irq_dependency_violations.len(),
        irq_dependency_violations,
        irq_conflict_count: irq_conflicts.len(),
        irq_conflicts,
        aa_deadlock_count: aa_deadlocks.len(),
        aa_deadlocks,
    };

    GlobalGraph {
        nodes: reverse_nodes,
        edges,
        report,
    }
}

fn intern_lock(
    node_ids: &mut BTreeMap<LockInfoArtifact, usize>,
    reverse_nodes: &mut Vec<LockInfoArtifact>,
    lock: &LockInfoArtifact,
) -> usize {
    if let Some(id) = node_ids.get(lock) {
        return *id;
    }

    let id = reverse_nodes.len();
    reverse_nodes.push(lock.clone());
    node_ids.insert(lock.clone(), id);
    id
}

fn find_cycles(
    reverse_nodes: &[LockInfoArtifact],
    adjacency: &BTreeMap<usize, BTreeSet<usize>>,
    graph_edges: &[GraphEdge],
) -> Vec<CycleReport> {
    let edge_origins = graph_edges
        .iter()
        .map(|edge| ((edge.from, edge.to), edge.origin.clone()))
        .collect::<BTreeMap<_, _>>();
    let sccs = tarjan_scc(reverse_nodes.len(), adjacency);
    let mut cycles = Vec::new();

    for scc in sccs {
        if scc.len() == 1 {
            let node = scc[0];
            let has_self_loop = adjacency
                .get(&node)
                .is_some_and(|neighbors| neighbors.contains(&node));
            if !has_self_loop {
                continue;
            }
        }

        if let Some(cycle_nodes) = find_cycle_in_scc(&scc, adjacency) {
            let mut steps = Vec::new();
            let mut locks = Vec::new();

            for window in cycle_nodes.windows(2) {
                let from = window[0];
                let to = window[1];
                let Some(origin) = edge_origins.get(&(from, to)).cloned() else {
                    continue;
                };
                locks.push(reverse_nodes[from].clone());
                steps.push(CycleStep {
                    from: reverse_nodes[from].clone(),
                    to: reverse_nodes[to].clone(),
                    origin,
                });
            }

            if is_blocking_cycle(&steps) {
                cycles.push(CycleReport { locks, steps });
            }
        }
    }

    cycles
}

/// Computes SCCs in the global lock graph.
fn tarjan_scc(node_count: usize, adjacency: &BTreeMap<usize, BTreeSet<usize>>) -> Vec<Vec<usize>> {
    #[derive(Default)]
    struct TarjanState {
        index: usize,
        stack: Vec<usize>,
        on_stack: BTreeSet<usize>,
        indices: BTreeMap<usize, usize>,
        lowlinks: BTreeMap<usize, usize>,
        components: Vec<Vec<usize>>,
    }

    fn strong_connect(
        node: usize,
        adjacency: &BTreeMap<usize, BTreeSet<usize>>,
        state: &mut TarjanState,
    ) {
        state.indices.insert(node, state.index);
        state.lowlinks.insert(node, state.index);
        state.index += 1;
        state.stack.push(node);
        state.on_stack.insert(node);

        if let Some(neighbors) = adjacency.get(&node) {
            for &neighbor in neighbors {
                if !state.indices.contains_key(&neighbor) {
                    strong_connect(neighbor, adjacency, state);
                    let lowlink = state.lowlinks[&node].min(state.lowlinks[&neighbor]);
                    state.lowlinks.insert(node, lowlink);
                } else if state.on_stack.contains(&neighbor) {
                    let lowlink = state.lowlinks[&node].min(state.indices[&neighbor]);
                    state.lowlinks.insert(node, lowlink);
                }
            }
        }

        if state.lowlinks[&node] == state.indices[&node] {
            let mut component = Vec::new();
            while let Some(stack_node) = state.stack.pop() {
                state.on_stack.remove(&stack_node);
                component.push(stack_node);
                if stack_node == node {
                    break;
                }
            }
            state.components.push(component);
        }
    }

    let mut state = TarjanState::default();
    for node in 0..node_count {
        if !state.indices.contains_key(&node) {
            strong_connect(node, adjacency, &mut state);
        }
    }

    state.components
}

/// Extracts one witness cycle path from an SCC when possible.
fn find_cycle_in_scc(
    scc: &[usize],
    adjacency: &BTreeMap<usize, BTreeSet<usize>>,
) -> Option<Vec<usize>> {
    let scc_set = scc.iter().copied().collect::<BTreeSet<_>>();
    let start = *scc.first()?;

    if adjacency
        .get(&start)
        .is_some_and(|neighbors| neighbors.contains(&start))
    {
        return Some(vec![start, start]);
    }

    for &neighbor in adjacency
        .get(&start)?
        .iter()
        .filter(|node| scc_set.contains(node))
    {
        let mut queue = VecDeque::from([neighbor]);
        let mut parents = BTreeMap::<usize, usize>::new();
        let mut visited = BTreeSet::from([neighbor]);

        while let Some(node) = queue.pop_front() {
            if node == start {
                let mut reverse_path = vec![start];
                let mut current = start;
                while current != neighbor {
                    current = parents[&current];
                    reverse_path.push(current);
                }
                reverse_path.reverse();
                let mut path = vec![start];
                path.extend(reverse_path);
                return Some(path);
            }

            if let Some(nexts) = adjacency.get(&node) {
                for &next in nexts.iter().filter(|next| scc_set.contains(next)) {
                    if visited.insert(next) {
                        parents.insert(next, node);
                        queue.push_back(next);
                    }
                }
            }
        }
    }

    None
}

fn format_cycle(cycle: &CycleReport) -> String {
    if cycle.steps.is_empty() {
        return "<empty>".into();
    }

    let mut parts = Vec::new();
    parts.push(short_lock_label(&cycle.steps[0].from));
    for step in &cycle.steps {
        parts.push(short_lock_label(&step.to));
    }
    parts.join(" -> ")
}

fn short_lock_label(lock: &LockInfoArtifact) -> String {
    format!("{}({})", lock.primitive, lock.acquire)
}

fn find_atomic_mode_violations(crates: &[AnalysisArtifact]) -> Vec<AtomicModeViolationReport> {
    let mut violations = Vec::new();
    let mut seen = BTreeSet::new();

    for crate_artifact in crates {
        for function in &crate_artifact.functions {
            for edge in &function.lock_edges {
                if !is_atomic_mode_violation_edge(&edge.from, &edge.to) {
                    continue;
                }

                let key = (
                    crate_artifact.package_name.clone(),
                    function.def_path.clone(),
                    edge.context.clone(),
                    edge.location.clone(),
                    edge.from.class.clone(),
                    edge.from.primitive.clone(),
                    edge.from.acquire.clone(),
                    edge.to.class.clone(),
                    edge.to.primitive.clone(),
                    edge.to.acquire.clone(),
                );
                if !seen.insert(key) {
                    continue;
                }

                violations.push(AtomicModeViolationReport {
                    kind: "sleeping_lock_in_atomic_context".into(),
                    crate_name: crate_artifact.package_name.clone(),
                    function: function.def_path.clone(),
                    context: edge.context.clone(),
                    held_lock: edge.from.clone(),
                    sleeping_lock: edge.to.clone(),
                    location: edge.location.clone(),
                });
            }
        }
    }

    violations
}

fn lock_class_label(lock_class: &LockClassKey) -> String {
    lock_class.to_string()
}

fn build_global_lock_usage_states(
    crates: &[AnalysisArtifact],
) -> BTreeMap<LockInfoArtifact, AggregatedLockUsageState> {
    let mut usage_states = BTreeMap::<LockInfoArtifact, AggregatedLockUsageState>::new();
    for crate_artifact in crates {
        for function in &crate_artifact.functions {
            for state in &function.lock_usage_states {
                let entry = usage_states
                    .entry(state.lock.clone())
                    .or_insert_with(|| empty_aggregated_lock_usage_state(state.lock.clone()));
                merge_aggregated_lock_usage_state(entry, crate_artifact, function, state);
            }
        }
    }

    usage_states
}

fn empty_aggregated_lock_usage_state(lock: LockInfoArtifact) -> AggregatedLockUsageState {
    AggregatedLockUsageState {
        lock,
        bits: LockUsageBitsArtifact::default(),
        first_hardirq_use: None,
        first_softirq_use: None,
        first_hardirq_enabled_use: None,
        first_hardirq_disabled_use: None,
        first_softirq_enabled_use: None,
        first_softirq_disabled_use: None,
    }
}

fn merge_aggregated_lock_usage_state(
    target: &mut AggregatedLockUsageState,
    crate_artifact: &AnalysisArtifact,
    function: &analysis::FunctionArtifact,
    state: &LockUsageStateArtifact,
) {
    target.bits.used_in_hardirq |= state.bits.used_in_hardirq;
    target.bits.used_in_softirq |= state.bits.used_in_softirq;
    target.bits.used_with_hardirq_enabled |= state.bits.used_with_hardirq_enabled;
    target.bits.used_with_hardirq_disabled |= state.bits.used_with_hardirq_disabled;
    target.bits.used_with_softirq_enabled |= state.bits.used_with_softirq_enabled;
    target.bits.used_with_softirq_disabled |= state.bits.used_with_softirq_disabled;

    merge_usage_site(
        &mut target.first_hardirq_use,
        crate_artifact,
        function,
        &state.lock,
        state.first_hardirq_use.as_ref(),
    );
    merge_usage_site(
        &mut target.first_softirq_use,
        crate_artifact,
        function,
        &state.lock,
        state.first_softirq_use.as_ref(),
    );
    merge_usage_site(
        &mut target.first_hardirq_enabled_use,
        crate_artifact,
        function,
        &state.lock,
        state.first_hardirq_enabled_use.as_ref(),
    );
    merge_usage_site(
        &mut target.first_hardirq_disabled_use,
        crate_artifact,
        function,
        &state.lock,
        state.first_hardirq_disabled_use.as_ref(),
    );
    merge_usage_site(
        &mut target.first_softirq_enabled_use,
        crate_artifact,
        function,
        &state.lock,
        state.first_softirq_enabled_use.as_ref(),
    );
    merge_usage_site(
        &mut target.first_softirq_disabled_use,
        crate_artifact,
        function,
        &state.lock,
        state.first_softirq_disabled_use.as_ref(),
    );
}

fn merge_usage_site(
    target: &mut Option<IrqUseSite>,
    crate_artifact: &AnalysisArtifact,
    function: &analysis::FunctionArtifact,
    lock: &LockInfoArtifact,
    source: Option<&LockUsageSiteArtifact>,
) {
    if target.is_some() {
        return;
    }
    let Some(source) = source else {
        return;
    };
    *target = Some(IrqUseSite {
        crate_name: crate_artifact.package_name.clone(),
        function: function.def_path.clone(),
        context: source.context.clone(),
        acquire: lock.acquire.clone(),
        location: source.location.clone(),
    });
}

fn find_single_lock_irq_violations(
    usage_states: &BTreeMap<LockInfoArtifact, AggregatedLockUsageState>,
) -> Vec<SingleLockIrqViolationReport> {
    let mut violations = Vec::new();

    for state in usage_states.values() {
        if state.bits.used_in_hardirq && state.bits.used_with_hardirq_enabled {
            violations.push(SingleLockIrqViolationReport {
                kind: "hardirq_safe_vs_unsafe".into(),
                class: lock_class_label(&state.lock.class),
                primitive: state.lock.primitive.clone(),
                acquire: state.lock.acquire.clone(),
                safe_site: state.first_hardirq_use.clone(),
                unsafe_site: state.first_hardirq_enabled_use.clone(),
            });
        }
        if state.bits.used_in_softirq && state.bits.used_with_softirq_enabled {
            violations.push(SingleLockIrqViolationReport {
                kind: "softirq_safe_vs_unsafe".into(),
                class: lock_class_label(&state.lock.class),
                primitive: state.lock.primitive.clone(),
                acquire: state.lock.acquire.clone(),
                safe_site: state.first_softirq_use.clone(),
                unsafe_site: state.first_softirq_enabled_use.clone(),
            });
        }
    }

    violations
}

fn find_irq_dependency_violations(
    edges: &[GraphEdge],
    nodes: &[LockInfoArtifact],
    usage_states: &BTreeMap<LockInfoArtifact, AggregatedLockUsageState>,
) -> Vec<IrqDependencyViolationReport> {
    let mut violations = Vec::new();
    let mut seen = BTreeSet::new();

    for edge in edges {
        let from = &nodes[edge.from];
        let to = &nodes[edge.to];
        let Some(from_state) = usage_states.get(from) else {
            continue;
        };
        let Some(to_state) = usage_states.get(to) else {
            continue;
        };

        if from_state.bits.used_in_hardirq && to_state.bits.used_with_hardirq_enabled {
            let key = (
                "hardirq_safe_to_unsafe".to_string(),
                from.class.clone(),
                from.acquire.clone(),
                to.class.clone(),
                to.acquire.clone(),
            );
            if seen.insert(key) {
                violations.push(IrqDependencyViolationReport {
                    kind: "hardirq_safe_to_unsafe".into(),
                    from: from.clone(),
                    to: to.clone(),
                    from_safe_site: from_state.first_hardirq_use.clone(),
                    to_unsafe_site: to_state.first_hardirq_enabled_use.clone(),
                    witness_edge_origin: edge.origin.clone(),
                });
            }
        }
        if from_state.bits.used_in_softirq && to_state.bits.used_with_softirq_enabled {
            let key = (
                "softirq_safe_to_unsafe".to_string(),
                from.class.clone(),
                from.acquire.clone(),
                to.class.clone(),
                to.acquire.clone(),
            );
            if seen.insert(key) {
                violations.push(IrqDependencyViolationReport {
                    kind: "softirq_safe_to_unsafe".into(),
                    from: from.clone(),
                    to: to.clone(),
                    from_safe_site: from_state.first_softirq_use.clone(),
                    to_unsafe_site: to_state.first_softirq_enabled_use.clone(),
                    witness_edge_origin: edge.origin.clone(),
                });
            }
        }
    }

    violations
}

fn legacy_irq_conflicts_from_single_lock_violations(
    violations: &[SingleLockIrqViolationReport],
) -> Vec<IrqConflictReport> {
    let mut conflicts = Vec::new();

    for violation in violations {
        let Some(safe_site) = &violation.safe_site else {
            continue;
        };
        let Some(unsafe_site) = &violation.unsafe_site else {
            continue;
        };
        conflicts.push(IrqConflictReport {
            class: violation.class.clone(),
            primitive: violation.primitive.clone(),
            acquire: violation.acquire.clone(),
            interrupt_sites: vec![safe_site.clone()],
            interruptible_sites: vec![unsafe_site.clone()],
        });
    }

    conflicts
}

/// Derives AA-style reports from self-loops and IRQ reentry conflicts.
fn find_aa_deadlocks(
    nodes: &[LockInfoArtifact],
    edges: &[GraphEdge],
    irq_conflicts: &[IrqConflictReport],
) -> Vec<AaDeadlockReport> {
    let mut reports = Vec::new();
    let mut seen_self = BTreeSet::new();

    for edge in edges {
        if edge.from != edge.to {
            continue;
        }
        let lock = &nodes[edge.from];
        if !seen_self.insert((
            lock.class.clone(),
            lock.primitive.clone(),
            lock.acquire.clone(),
        )) {
            continue;
        }
        reports.push(AaDeadlockReport {
            kind: "self_lock".into(),
            class: lock_class_label(&lock.class),
            primitive: lock.primitive.clone(),
            sites: vec![AaDeadlockSite {
                function: edge.origin.function.clone(),
                context: edge.origin.context.clone(),
                acquire: lock.acquire.clone(),
                location: edge.origin.location.clone(),
            }],
        });
    }

    for conflict in irq_conflicts {
        let mut sites = Vec::new();
        if let Some(site) = conflict.interrupt_sites.first() {
            sites.push(AaDeadlockSite {
                function: site.function.clone(),
                context: site.context.clone(),
                acquire: site.acquire.clone(),
                location: site.location.clone(),
            });
        }
        if let Some(site) = conflict.interruptible_sites.first() {
            sites.push(AaDeadlockSite {
                function: site.function.clone(),
                context: site.context.clone(),
                acquire: site.acquire.clone(),
                location: site.location.clone(),
            });
        }
        reports.push(AaDeadlockReport {
            kind: "irq_reentry".into(),
            class: conflict.class.clone(),
            primitive: conflict.primitive.clone(),
            sites,
        });
    }

    reports
}

fn format_site_location(location: Option<&SourceLocation>) -> String {
    location
        .map(|location| format!("at {}:{}:{}", location.file, location.line, location.column))
        .unwrap_or_else(|| "at <unknown>".into())
}

fn is_deadlock_relevant_edge(from: &LockInfoArtifact, to: &LockInfoArtifact) -> bool {
    !(is_shared_read_lock(from) && is_shared_read_lock(to))
}

fn is_atomic_mode_violation_edge(from: &LockInfoArtifact, to: &LockInfoArtifact) -> bool {
    is_atomic_mode_lock(from) && is_sleeping_lock(to)
}

fn is_atomic_mode_lock(lock: &LockInfoArtifact) -> bool {
    matches!(lock.primitive.as_str(), "SpinLock" | "RwLock")
}

fn is_sleeping_lock(lock: &LockInfoArtifact) -> bool {
    matches!(lock.primitive.as_str(), "Mutex" | "RwMutex")
}

fn is_shared_read_lock(lock: &LockInfoArtifact) -> bool {
    lock.acquire == "read" && matches!(lock.primitive.as_str(), "RwLock" | "RwMutex")
}

fn is_blocking_cycle(steps: &[CycleStep]) -> bool {
    if steps.is_empty() {
        return false;
    }

    for index in 0..steps.len() {
        let current = &steps[index];
        let next = &steps[(index + 1) % steps.len()];
        if !same_lock_class(&current.to, &next.from) {
            return false;
        }
        if !modes_conflict(&current.to, &next.from) {
            return false;
        }
    }

    true
}

fn same_lock_class(left: &LockInfoArtifact, right: &LockInfoArtifact) -> bool {
    left.class == right.class && left.primitive == right.primitive
}

fn modes_conflict(target: &LockInfoArtifact, held: &LockInfoArtifact) -> bool {
    match target.primitive.as_str() {
        "RwLock" | "RwMutex" => !(target.acquire == "read" && held.acquire == "read"),
        _ => true,
    }
}

fn render_dot(graph: &GlobalGraph) -> String {
    let mut dot = String::from("digraph lockdep {\n  rankdir=LR;\n  node [shape=box];\n");

    for (index, lock) in graph.nodes.iter().enumerate() {
        let label = escape_dot(&format!(
            "{}\\n{}\\n{}",
            short_lock_label(lock),
            lock.guard_behavior,
            lock_class_label(&lock.class)
        ));
        dot.push_str(&format!("  n{index} [label=\"{label}\"];\n"));
    }

    for edge in &graph.edges {
        let mut edge_label = format!("{}\\n{}", edge.origin.context, edge.origin.function);
        if let Some(location) = &edge.origin.location {
            edge_label.push_str(&format!(
                "\\n{}:{}:{}",
                location.file, location.line, location.column
            ));
        }
        dot.push_str(&format!(
            "  n{} -> n{} [label=\"{}\"];\n",
            edge.from,
            edge.to,
            escape_dot(&edge_label)
        ));
    }

    dot.push_str("}\n");
    dot
}

fn escape_dot(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}
