// SPDX-License-Identifier: MPL-2.0

use std::collections::HashSet;

use super::*;

#[derive(Clone)]
pub(super) struct InboundCallableBindings {
    pub(super) caller: DefId,
    pub(super) bindings: BTreeMap<usize, place::CallableTarget>,
}

pub(super) fn collect_direct_calls<'tcx>(tcx: TyCtxt<'tcx>) -> HashMap<DefId, HashSet<DefId>> {
    let mut graph = HashMap::<DefId, HashSet<DefId>>::new();

    for local_def_id in tcx.mir_keys(()) {
        let caller = local_def_id.to_def_id();
        if !tcx.def_kind(caller).is_fn_like() {
            continue;
        }

        let body = tcx.optimized_mir(caller);
        let mut callees = HashSet::new();
        for block_data in body.basic_blocks.iter() {
            let Some(terminator) = &block_data.terminator else {
                continue;
            };
            let TerminatorKind::Call { func, .. } = &terminator.kind else {
                continue;
            };
            let Some(callee) = place::resolve_called_def_id(body, func.clone()) else {
                continue;
            };
            if callee.is_local() && tcx.def_kind(callee).is_fn_like() {
                callees.insert(callee);
            }
        }
        graph.insert(caller, callees);
    }

    graph
}

pub(super) fn propagate_contexts(
    entry_contexts: &HashMap<DefId, Vec<context::ContextKind>>,
    direct_calls: &HashMap<DefId, HashSet<DefId>>,
) -> HashMap<DefId, Vec<context::ContextKind>> {
    let mut propagated = entry_contexts
        .iter()
        .map(|(def_id, contexts)| (*def_id, contexts.iter().copied().collect::<BTreeSet<_>>()))
        .collect::<HashMap<_, _>>();

    let mut queue = VecDeque::new();
    for (&def_id, contexts) in &propagated {
        for &context in contexts {
            queue.push_back((def_id, context));
        }
    }

    while let Some((caller, context)) = queue.pop_front() {
        let Some(callees) = direct_calls.get(&caller) else {
            continue;
        };
        for &callee in callees {
            let entry = propagated.entry(callee).or_default();
            if entry.insert(context) {
                queue.push_back((callee, context));
            }
        }
    }

    propagated
        .into_iter()
        .map(|(def_id, contexts)| {
            let mut contexts = contexts.into_iter().collect::<Vec<_>>();
            if contexts.is_empty() {
                contexts.push(context::ContextKind::Task);
            }
            (def_id, contexts)
        })
        .collect()
}

pub(super) fn compute_summary_entry_locks(
    analyses: &[FunctionAnalysis],
) -> HashMap<DefId, BTreeSet<LockInfoArtifact>> {
    let inbound_callable_bindings = build_inbound_callable_bindings(analyses);
    let initial = analyses
        .iter()
        .map(|analysis| (analysis.def_id, analysis.entry_locks.clone()))
        .collect::<HashMap<_, _>>();
    compute_summary_locks(analyses, initial, &inbound_callable_bindings, |callsite| {
        callsite.held_locks.is_empty()
    })
}

pub(super) fn compute_summary_return_locks(
    analyses: &[FunctionAnalysis],
) -> HashMap<DefId, BTreeSet<LockInfoArtifact>> {
    let inbound_callable_bindings = build_inbound_callable_bindings(analyses);
    let return_place = Local::from_usize(0);
    let initial = analyses
        .iter()
        .map(|analysis| (analysis.def_id, analysis.return_locks.clone()))
        .collect::<HashMap<_, _>>();
    compute_summary_locks(analyses, initial, &inbound_callable_bindings, |callsite| {
        callsite.destination == Some(return_place)
    })
}

pub(super) fn build_inbound_callable_bindings(
    analyses: &[FunctionAnalysis],
) -> HashMap<DefId, Vec<InboundCallableBindings>> {
    analyses
        .iter()
        .flat_map(|analysis| {
            analysis.callsites.iter().filter_map(move |callsite| {
                callsite.direct_callee().map(|callee| {
                    (
                        callee,
                        InboundCallableBindings {
                            caller: analysis.def_id,
                            bindings: callsite.callable_arg_bindings.clone(),
                        },
                    )
                })
            })
        })
        .fold(
            HashMap::<DefId, Vec<InboundCallableBindings>>::new(),
            |mut map, (callee, bindings)| {
                map.entry(callee).or_default().push(bindings);
                map
            },
        )
}

pub(super) fn resolve_callable_arg_callees(
    function: DefId,
    arg_index: usize,
    inbound_callable_bindings: &HashMap<DefId, Vec<InboundCallableBindings>>,
) -> Vec<DefId> {
    fn visit(
        function: DefId,
        arg_index: usize,
        inbound_callable_bindings: &HashMap<DefId, Vec<InboundCallableBindings>>,
        visiting: &mut HashSet<(DefId, usize)>,
        resolved: &mut HashSet<DefId>,
    ) {
        if !visiting.insert((function, arg_index)) {
            return;
        }
        let Some(inbound) = inbound_callable_bindings.get(&function) else {
            visiting.remove(&(function, arg_index));
            return;
        };
        for inbound_binding in inbound {
            let Some(target) = inbound_binding.bindings.get(&arg_index) else {
                continue;
            };
            match *target {
                place::CallableTarget::Direct(def_id) => {
                    resolved.insert(def_id);
                }
                place::CallableTarget::Arg(parent_arg_index) => {
                    visit(
                        inbound_binding.caller,
                        parent_arg_index,
                        inbound_callable_bindings,
                        visiting,
                        resolved,
                    );
                }
            }
        }
        visiting.remove(&(function, arg_index));
    }

    let mut visiting = HashSet::new();
    let mut resolved = HashSet::new();
    visit(
        function,
        arg_index,
        inbound_callable_bindings,
        &mut visiting,
        &mut resolved,
    );
    resolved.into_iter().collect()
}

pub(super) fn resolve_callsite_callees(
    callsite: &CallSite,
    current_def_id: DefId,
    inbound_callable_bindings: &HashMap<DefId, Vec<InboundCallableBindings>>,
) -> Vec<DefId> {
    match callsite.target {
        CallTarget::Direct(callee) => vec![callee],
        CallTarget::CallableArg(index) => {
            resolve_callable_arg_callees(current_def_id, index, inbound_callable_bindings)
        }
    }
}

pub(super) fn finalize_function_analysis(
    mut analysis: FunctionAnalysis,
    summary_entry_locks: &HashMap<DefId, BTreeSet<LockInfoArtifact>>,
    inbound_callable_bindings: &HashMap<DefId, Vec<InboundCallableBindings>>,
) -> FunctionArtifact {
    let mut propagated_usage_states = BTreeMap::<LockInfoArtifact, LockUsageStateArtifact>::new();
    for state in analysis.lock_usage_states.drain(..) {
        let entry = propagated_usage_states
            .entry(state.lock.clone())
            .or_insert_with(|| empty_lock_usage_state(state.lock.clone()));
        merge_lock_usage_state(entry, state);
    }

    for callsite in &analysis.callsites {
        let callee_locks = summary_locks_for_callsite(
            callsite,
            analysis.def_id,
            summary_entry_locks,
            inbound_callable_bindings,
        );
        if callee_locks.is_empty() {
            continue;
        }
        for callee_lock in &callee_locks {
            let propagated_irq_state = callsite.irq_state.for_acquire(&callee_lock);
            record_lock_usage(
                &mut propagated_usage_states,
                callee_lock,
                &propagated_irq_state,
                LockUsageSiteArtifact {
                    context: propagated_irq_state.current_context().as_str().to_string(),
                    location: callsite.location.clone(),
                },
            );
        }
        for held in &callsite.held_locks {
            for callee_lock in &callee_locks {
                if is_deadlock_relevant_lock_pair(held, &callee_lock) {
                    analysis.lock_edges.push(LockEdgeArtifact {
                        context: callsite.context.as_str().to_string(),
                        from: held.clone(),
                        to: callee_lock.clone(),
                        location: callsite.location.clone(),
                    });
                }
            }
        }
    }
    place::dedup_edges(&mut analysis.lock_edges);
    let mut lock_usage_states =
        merge_lock_usage_states(propagated_usage_states.into_values().collect());
    lock_usage_states.sort_by(|left, right| {
        left.lock
            .class
            .cmp(&right.lock.class)
            .then(left.lock.primitive.cmp(&right.lock.primitive))
            .then(left.lock.acquire.cmp(&right.lock.acquire))
    });

    FunctionArtifact {
        def_path: analysis.def_path,
        location: analysis.location,
        contexts: analysis
            .contexts
            .iter()
            .map(|context| context.as_str().to_string())
            .collect(),
        lock_events: analysis.lock_events,
        lock_edges: analysis.lock_edges,
        lock_usage_states,
    }
}

fn compute_summary_locks(
    analyses: &[FunctionAnalysis],
    mut summaries: HashMap<DefId, BTreeSet<LockInfoArtifact>>,
    inbound_callable_bindings: &HashMap<DefId, Vec<InboundCallableBindings>>,
    include_callsite: impl Fn(&CallSite) -> bool,
) -> HashMap<DefId, BTreeSet<LockInfoArtifact>> {
    let mut changed = true;
    while changed {
        changed = false;
        for analysis in analyses {
            let mut merged = summaries.get(&analysis.def_id).cloned().unwrap_or_default();
            for callsite in analysis
                .callsites
                .iter()
                .filter(|callsite| include_callsite(callsite))
            {
                let previous_len = merged.len();
                merged.extend(summary_locks_for_callsite(
                    callsite,
                    analysis.def_id,
                    &summaries,
                    inbound_callable_bindings,
                ));
                changed |= merged.len() != previous_len;
            }
            summaries.insert(analysis.def_id, merged);
        }
    }
    summaries
}

fn summary_locks_for_callsite(
    callsite: &CallSite,
    current_def_id: DefId,
    summary_entry_locks: &HashMap<DefId, BTreeSet<LockInfoArtifact>>,
    inbound_callable_bindings: &HashMap<DefId, Vec<InboundCallableBindings>>,
) -> Vec<LockInfoArtifact> {
    resolve_callsite_callees(callsite, current_def_id, inbound_callable_bindings)
        .into_iter()
        .flat_map(|callee| {
            summary_entry_locks
                .get(&callee)
                .into_iter()
                .flat_map(|locks| locks.iter())
        })
        .filter_map(|lock| place::rebind_lock_info(lock, &callsite.arg_bindings))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(super) fn record_lock_usage(
    usage_states: &mut BTreeMap<LockInfoArtifact, LockUsageStateArtifact>,
    lock: &LockInfoArtifact,
    irq_state: &context::IrqExecutionState,
    site: LockUsageSiteArtifact,
) {
    let entry = usage_states
        .entry(lock.clone())
        .or_insert_with(|| empty_lock_usage_state(lock.clone()));
    apply_lock_usage(entry, irq_state, site);
}

fn empty_lock_usage_state(lock: LockInfoArtifact) -> LockUsageStateArtifact {
    LockUsageStateArtifact {
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

fn apply_lock_usage(
    state: &mut LockUsageStateArtifact,
    irq_state: &context::IrqExecutionState,
    site: LockUsageSiteArtifact,
) {
    if irq_state.hardirq_context_depth > 0 {
        state.bits.used_in_hardirq = true;
        if state.first_hardirq_use.is_none() {
            state.first_hardirq_use = Some(site.clone());
        }
    }
    if irq_state.softirq_context_depth > 0 {
        state.bits.used_in_softirq = true;
        if state.first_softirq_use.is_none() {
            state.first_softirq_use = Some(site.clone());
        }
    }
    if irq_state.hardirq_enabled {
        state.bits.used_with_hardirq_enabled = true;
        if state.first_hardirq_enabled_use.is_none() {
            state.first_hardirq_enabled_use = Some(site.clone());
        }
    } else {
        state.bits.used_with_hardirq_disabled = true;
        if state.first_hardirq_disabled_use.is_none() {
            state.first_hardirq_disabled_use = Some(site.clone());
        }
    }
    if irq_state.softirq_enabled {
        state.bits.used_with_softirq_enabled = true;
        if state.first_softirq_enabled_use.is_none() {
            state.first_softirq_enabled_use = Some(site);
        }
    } else {
        state.bits.used_with_softirq_disabled = true;
        if state.first_softirq_disabled_use.is_none() {
            state.first_softirq_disabled_use = Some(site);
        }
    }
}

pub(super) fn merge_lock_usage_states(
    states: Vec<LockUsageStateArtifact>,
) -> Vec<LockUsageStateArtifact> {
    let mut merged = BTreeMap::<LockInfoArtifact, LockUsageStateArtifact>::new();
    for state in states {
        let entry = merged
            .entry(state.lock.clone())
            .or_insert_with(|| empty_lock_usage_state(state.lock.clone()));
        merge_lock_usage_state(entry, state);
    }
    merged.into_values().collect()
}

fn merge_lock_usage_state(target: &mut LockUsageStateArtifact, source: LockUsageStateArtifact) {
    target.bits.used_in_hardirq |= source.bits.used_in_hardirq;
    target.bits.used_in_softirq |= source.bits.used_in_softirq;
    target.bits.used_with_hardirq_enabled |= source.bits.used_with_hardirq_enabled;
    target.bits.used_with_hardirq_disabled |= source.bits.used_with_hardirq_disabled;
    target.bits.used_with_softirq_enabled |= source.bits.used_with_softirq_enabled;
    target.bits.used_with_softirq_disabled |= source.bits.used_with_softirq_disabled;

    if target.first_hardirq_use.is_none() {
        target.first_hardirq_use = source.first_hardirq_use;
    }
    if target.first_softirq_use.is_none() {
        target.first_softirq_use = source.first_softirq_use;
    }
    if target.first_hardirq_enabled_use.is_none() {
        target.first_hardirq_enabled_use = source.first_hardirq_enabled_use;
    }
    if target.first_hardirq_disabled_use.is_none() {
        target.first_hardirq_disabled_use = source.first_hardirq_disabled_use;
    }
    if target.first_softirq_enabled_use.is_none() {
        target.first_softirq_enabled_use = source.first_softirq_enabled_use;
    }
    if target.first_softirq_disabled_use.is_none() {
        target.first_softirq_disabled_use = source.first_softirq_disabled_use;
    }
}

pub(super) fn is_deadlock_relevant_lock_pair(
    from: &LockInfoArtifact,
    to: &LockInfoArtifact,
) -> bool {
    !(is_shared_read_lock(from) && is_shared_read_lock(to))
}

fn is_shared_read_lock(lock: &LockInfoArtifact) -> bool {
    (lock.primitive == "RwLock" || lock.primitive == "RwMutex") && lock.acquire == "read"
}
