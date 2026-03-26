// SPDX-License-Identifier: MPL-2.0

//! MIR-based fact collection for the lockdep prototype.
//!
//! This module is responsible for:
//!
//! - identifying lock acquire/release operations in MIR;
//! - building per-function lock events and edges;
//! - recovering a minimal direct-call summary;
//! - recognizing IRQ entry contexts and propagating them through direct calls.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    env,
};

use rustc_middle::{
    mir::{Body, Local, Place, StatementKind, TerminatorKind},
    ty::TyCtxt,
};
use rustc_span::{
    def_id::{DefId, LOCAL_CRATE},
    source_map::Spanned,
};

use crate::model::{
    AnalysisArtifact, FunctionArtifact, LocalOriginKey, LockClassKey, LockEdgeArtifact,
    LockEventArtifact, LockInfoArtifact, LockUsageBitsArtifact, LockUsageSiteArtifact,
    LockUsageStateArtifact,
};

#[path = "collect/context.rs"]
mod context;
#[path = "collect/mir.rs"]
mod mir;
#[path = "collect/place.rs"]
mod place;
#[path = "collect/summary.rs"]
mod summary;

use self::{
    context::{ContextKind, IrqExecutionState, collect_entry_contexts},
    place::span_to_location,
    summary::{
        build_inbound_callable_bindings, collect_direct_calls, compute_summary_entry_locks,
        finalize_function_analysis, propagate_contexts, resolve_callsite_callees,
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
struct BlockState<'tcx> {
    aliases: BTreeMap<Local, Place<'tcx>>,
    guards: BTreeMap<Local, LockInfoArtifact>,
    irq_guard_locals: BTreeSet<Local>,
    irq_state: IrqExecutionState,
    local_origins: BTreeMap<Local, LocalOriginKey>,
}

impl<'tcx> BlockState<'tcx> {
    fn empty() -> Self {
        Self::with_context(ContextKind::Task)
    }

    fn with_context(context: ContextKind) -> Self {
        Self {
            aliases: BTreeMap::new(),
            guards: BTreeMap::new(),
            irq_guard_locals: BTreeSet::new(),
            irq_state: IrqExecutionState::from_context(context),
            local_origins: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
enum CallEffect<'tcx> {
    Alias {
        destination: Local,
        source: Place<'tcx>,
    },
    Acquire {
        destination: Local,
        lock: LockInfoArtifact,
    },
    Release {
        local: Local,
    },
}

#[derive(Clone, Debug)]
enum CallTarget {
    Direct(DefId),
    CallableArg(usize),
}

#[derive(Clone, Debug)]
struct CallSite {
    target: CallTarget,
    destination: Option<Local>,
    arg_bindings: BTreeMap<usize, LockClassKey>,
    callable_arg_bindings: BTreeMap<usize, place::CallableTarget>,
    held_locks: Vec<LockInfoArtifact>,
    context: ContextKind,
    irq_state: IrqExecutionState,
    location: Option<crate::model::SourceLocation>,
}

impl CallSite {
    fn direct_callee(&self) -> Option<DefId> {
        match self.target {
            CallTarget::Direct(def_id) => Some(def_id),
            CallTarget::CallableArg(_) => None,
        }
    }
}

#[derive(Clone, Debug)]
struct BodyAnalysis {
    lock_events: Vec<LockEventArtifact>,
    lock_edges: Vec<LockEdgeArtifact>,
    lock_usage_states: Vec<LockUsageStateArtifact>,
    entry_locks: BTreeSet<LockInfoArtifact>,
    return_locks: BTreeSet<LockInfoArtifact>,
    callsites: Vec<CallSite>,
}

#[derive(Clone, Debug)]
struct FunctionAnalysis {
    def_id: DefId,
    def_path: String,
    location: Option<crate::model::SourceLocation>,
    contexts: Vec<ContextKind>,
    lock_events: Vec<LockEventArtifact>,
    lock_edges: Vec<LockEdgeArtifact>,
    lock_usage_states: Vec<LockUsageStateArtifact>,
    entry_locks: BTreeSet<LockInfoArtifact>,
    return_locks: BTreeSet<LockInfoArtifact>,
    callsites: Vec<CallSite>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct ConfiguredIrqEntry {
    function: String,
    context: String,
    #[serde(default)]
    callback_arg_index: usize,
}

pub fn collect_artifact<'tcx>(tcx: TyCtxt<'tcx>) -> AnalysisArtifact {
    let crate_name = tcx.crate_name(LOCAL_CRATE).to_string();
    let package_name = env::var("CARGO_PKG_NAME").unwrap_or_else(|_| crate_name.clone());
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let target = env::var("TARGET").ok();
    let is_primary_package = env::var("CARGO_PRIMARY_PACKAGE").is_ok();

    let entry_contexts = collect_entry_contexts(tcx);
    let direct_calls = collect_direct_calls(tcx);
    let (_propagated_contexts, analyses) =
        analyze_functions_with_propagated_contexts(tcx, &entry_contexts, &direct_calls);
    let summary_entry_locks = compute_summary_entry_locks(&analyses);
    let inbound_callable_bindings = build_inbound_callable_bindings(&analyses);

    let mut functions = analyses
        .into_iter()
        .map(|analysis| {
            finalize_function_analysis(analysis, &summary_entry_locks, &inbound_callable_bindings)
        })
        .collect::<Vec<_>>();
    functions.sort_by(|left, right| left.def_path.cmp(&right.def_path));

    AnalysisArtifact {
        crate_name,
        package_name,
        manifest_dir,
        target,
        is_primary_package,
        functions,
    }
}

fn analyze_functions_with_propagated_contexts<'tcx>(
    tcx: TyCtxt<'tcx>,
    entry_contexts: &HashMap<DefId, Vec<ContextKind>>,
    direct_calls: &HashMap<DefId, HashSet<DefId>>,
) -> (HashMap<DefId, Vec<ContextKind>>, Vec<FunctionAnalysis>) {
    let mut seeded_contexts = normalize_context_map(entry_contexts);

    loop {
        let propagated_contexts = propagate_contexts(
            &seeded_contexts
                .iter()
                .map(|(def_id, contexts)| (*def_id, contexts.iter().copied().collect()))
                .collect(),
            direct_calls,
        );
        let analyses = analyze_functions(tcx, &propagated_contexts);
        let callsite_contexts = collect_callsite_contexts(&analyses);
        let next_seeded_contexts = merge_context_maps(entry_contexts, &callsite_contexts);

        if next_seeded_contexts == seeded_contexts {
            return (propagated_contexts, analyses);
        }

        seeded_contexts = next_seeded_contexts;
    }
}

fn collect_callsite_contexts(analyses: &[FunctionAnalysis]) -> HashMap<DefId, Vec<ContextKind>> {
    let mut contexts = HashMap::<DefId, BTreeSet<ContextKind>>::new();
    let inbound_callable_bindings = build_inbound_callable_bindings(analyses);

    for analysis in analyses {
        for callsite in &analysis.callsites {
            for callee in
                resolve_callsite_callees(callsite, analysis.def_id, &inbound_callable_bindings)
            {
                contexts.entry(callee).or_default().insert(callsite.context);
            }
        }
    }

    contexts
        .into_iter()
        .map(|(def_id, contexts)| (def_id, contexts.into_iter().collect()))
        .collect()
}

fn merge_context_maps(
    base: &HashMap<DefId, Vec<ContextKind>>,
    extra: &HashMap<DefId, Vec<ContextKind>>,
) -> HashMap<DefId, BTreeSet<ContextKind>> {
    let mut merged = normalize_context_map(base);
    for (&def_id, contexts) in extra {
        let entry = merged.entry(def_id).or_default();
        entry.extend(contexts.iter().copied());
    }
    merged
}

fn normalize_context_map(
    contexts: &HashMap<DefId, Vec<ContextKind>>,
) -> HashMap<DefId, BTreeSet<ContextKind>> {
    contexts
        .iter()
        .map(|(def_id, contexts)| (*def_id, contexts.iter().copied().collect()))
        .collect()
}

fn analyze_functions<'tcx>(
    tcx: TyCtxt<'tcx>,
    propagated_contexts: &HashMap<DefId, Vec<ContextKind>>,
) -> Vec<FunctionAnalysis> {
    let mut return_lock_summaries = HashMap::<DefId, BTreeSet<LockInfoArtifact>>::new();

    loop {
        let analyses = tcx
            .mir_keys(())
            .iter()
            .filter(|local_def_id| tcx.def_kind(local_def_id.to_def_id()).is_fn_like())
            .map(|local_def_id| {
                let def_id = local_def_id.to_def_id();
                let body = tcx.optimized_mir(def_id);
                let location = span_to_location(tcx, tcx.def_span(def_id));
                let contexts = propagated_contexts
                    .get(&def_id)
                    .cloned()
                    .unwrap_or_else(|| vec![ContextKind::Task]);

                let mut lock_events = Vec::new();
                let mut lock_edges = Vec::new();
                let mut lock_usage_states = Vec::new();
                let mut entry_locks = BTreeSet::new();
                let mut return_locks = BTreeSet::new();
                let mut callsites = Vec::new();

                for context in &contexts {
                    let analysis =
                        analyze_body(tcx, def_id, body, *context, &return_lock_summaries);
                    lock_events.extend(analysis.lock_events);
                    lock_edges.extend(analysis.lock_edges);
                    lock_usage_states.extend(analysis.lock_usage_states);
                    entry_locks.extend(analysis.entry_locks);
                    return_locks.extend(analysis.return_locks);
                    callsites.extend(analysis.callsites);
                }

                FunctionAnalysis {
                    def_id,
                    def_path: tcx.def_path_str(def_id),
                    location,
                    contexts,
                    lock_events,
                    lock_edges,
                    lock_usage_states,
                    entry_locks,
                    return_locks,
                    callsites,
                }
            })
            .collect::<Vec<_>>();

        let new_return_lock_summaries = summary::compute_summary_return_locks(&analyses);

        if new_return_lock_summaries == return_lock_summaries {
            return analyses;
        }

        return_lock_summaries = new_return_lock_summaries;
    }
}

fn analyze_body<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_def_id: DefId,
    body: &Body<'tcx>,
    base_context: ContextKind,
    return_lock_summaries: &HashMap<DefId, BTreeSet<LockInfoArtifact>>,
) -> BodyAnalysis {
    let in_states = mir::compute_block_states(
        tcx,
        current_def_id,
        body,
        base_context,
        return_lock_summaries,
    );
    let mut events = Vec::new();
    let mut edges = Vec::new();
    let mut usage_states = BTreeMap::<LockInfoArtifact, LockUsageStateArtifact>::new();
    let mut entry_locks = BTreeSet::new();
    let mut return_locks = BTreeSet::new();
    let mut callsites = Vec::new();

    for (block, block_data) in body.basic_blocks.iter_enumerated() {
        let mut state = in_states[block.as_usize()]
            .clone()
            .unwrap_or_else(BlockState::empty);

        for statement in &block_data.statements {
            mir::apply_statement(
                tcx,
                current_def_id,
                body,
                &mut state,
                statement
                    .kind
                    .as_assign()
                    .map(|(place, rvalue)| (*place, rvalue)),
            );

            if let StatementKind::StorageDead(local) = statement.kind {
                place::release_local(&mut state, local, statement.kind.name());
            }
        }

        if let Some(terminator) = &block_data.terminator {
            if let Some(callsite) = mir::collect_callsite(
                tcx,
                current_def_id,
                body,
                &state,
                base_context,
                Some(&terminator.kind),
                span_to_location(tcx, terminator.source_info.span),
            ) {
                callsites.push(callsite);
            }
            if let Some(effect) = mir::classify_call_effect(
                tcx,
                current_def_id,
                body,
                &state,
                Some(&terminator.kind),
                return_lock_summaries,
            ) {
                mir::emit_effect(
                    &mut events,
                    &mut edges,
                    &mut usage_states,
                    &mut entry_locks,
                    body,
                    &state,
                    base_context,
                    effect.clone(),
                    span_to_location(tcx, terminator.source_info.span),
                );
                mir::apply_effect(&mut state, &effect);
            } else if let TerminatorKind::Drop { place, .. } = &terminator.kind
                && let Some(local) = place.as_local()
            {
                if let Some(lock) = state.guards.get(&local).cloned() {
                    events.push(LockEventArtifact {
                        kind: "release".into(),
                        context: context::current_context(&state, base_context)
                            .as_str()
                            .to_string(),
                        lock,
                        guard_local: Some(place::format_local(body, local)),
                        location: span_to_location(tcx, terminator.source_info.span),
                    });
                }
                place::release_local(&mut state, local, "Drop");
            } else if matches!(terminator.kind, TerminatorKind::Return) {
                return_locks.extend(state.guards.values().cloned());
            }
        }
    }

    place::dedup_edges(&mut edges);
    BodyAnalysis {
        lock_events: events,
        lock_edges: edges,
        lock_usage_states: usage_states.into_values().collect(),
        entry_locks,
        return_locks,
        callsites,
    }
}
