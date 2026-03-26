// SPDX-License-Identifier: MPL-2.0

use rustc_middle::{
    mir::{BasicBlock, Body, Local, Operand, Place, Rvalue, StatementKind, TerminatorKind},
    ty::{self, AdtDef, GenericArgsRef, Ty, TyCtxt},
};

use super::*;
use crate::SourceLocation;

pub(super) fn compute_block_states<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_def_id: DefId,
    body: &Body<'tcx>,
    base_context: context::ContextKind,
    return_lock_summaries: &HashMap<DefId, BTreeSet<LockInfoArtifact>>,
) -> Vec<Option<BlockState<'tcx>>> {
    let mut in_states = vec![None; body.basic_blocks.len()];
    in_states[BasicBlock::from_usize(0).as_usize()] = Some(context::softirq_entry_state_for_body(
        tcx,
        body,
        base_context,
    ));

    let predecessors = body.basic_blocks.predecessors().clone();
    let mut queue = VecDeque::from([BasicBlock::from_usize(0)]);

    while let Some(block) = queue.pop_front() {
        let mut state = in_states[block.as_usize()]
            .clone()
            .unwrap_or_else(BlockState::empty);

        let block_data = &body[block];
        for statement in &block_data.statements {
            apply_statement(
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
            if let Some(effect) = classify_call_effect(
                tcx,
                current_def_id,
                body,
                &state,
                Some(&terminator.kind),
                return_lock_summaries,
            ) {
                apply_effect(&mut state, &effect);
            } else if let TerminatorKind::Drop { place, .. } = &terminator.kind
                && let Some(local) = place.as_local()
            {
                place::release_local(&mut state, local, "Drop");
            }

            for successor in terminator.successors() {
                let joined = if let Some(existing) = &in_states[successor.as_usize()] {
                    join_states(existing, &state)
                } else if predecessors[successor].len() <= 1 {
                    state.clone()
                } else {
                    let pred_states = predecessors[successor]
                        .iter()
                        .filter_map(|predecessor| {
                            in_states[predecessor.as_usize()].as_ref().cloned()
                        })
                        .collect::<Vec<_>>();
                    join_many_states(&pred_states).unwrap_or_else(BlockState::empty)
                };

                if in_states[successor.as_usize()].as_ref() != Some(&joined) {
                    in_states[successor.as_usize()] = Some(joined);
                    queue.push_back(successor);
                }
            }
        }
    }

    in_states
}

pub(super) fn apply_statement<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_def_id: DefId,
    body: &Body<'tcx>,
    state: &mut BlockState<'tcx>,
    assign: Option<(Place<'tcx>, &Rvalue<'tcx>)>,
) {
    let Some((place, rvalue)) = assign else {
        return;
    };
    let Some(local) = place.as_local() else {
        return;
    };

    state.aliases.remove(&local);
    state.guards.remove(&local);
    state.irq_guard_locals.remove(&local);
    state.local_origins.remove(&local);

    match rvalue {
        Rvalue::Ref(_, _, source) => {
            state
                .aliases
                .insert(local, place::resolve_alias(state, *source));
            state.local_origins.insert(
                local,
                LocalOriginKey::RefOfPlace {
                    base: Box::new(place::place_root_key(
                        tcx,
                        current_def_id,
                        body,
                        state,
                        source.local,
                    )),
                },
            );
        }
        Rvalue::Use(operand) => {
            place::transfer_guard_move(state, local, operand);
            if let Some(source) = operand.place() {
                let source = place::resolve_alias(state, source);
                if source.as_local().is_some() {
                    state.aliases.insert(local, source);
                }
            }
            state.local_origins.insert(
                local,
                place::classify_operand_origin(tcx, current_def_id, body, state, operand),
            );
        }
        Rvalue::Cast(_, operand, _) => {
            if let Some(source) = operand.place() {
                let source = place::resolve_alias(state, source);
                if source.as_local().is_some() {
                    state.aliases.insert(local, source);
                }
            }
            state.local_origins.insert(
                local,
                place::classify_operand_origin(tcx, current_def_id, body, state, operand),
            );
        }
        Rvalue::Aggregate(_, _) => {
            state.local_origins.insert(
                local,
                LocalOriginKey::AggregateTemp {
                    ty: place::type_key(body.local_decls[local].ty),
                },
            );
        }
        _ => {
            let _ = tcx;
            let _ = body;
            state.local_origins.insert(local, LocalOriginKey::Unknown);
        }
    }

    context::refresh_irq_state(state);
}

pub(super) fn classify_call_effect<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_def_id: DefId,
    body: &Body<'tcx>,
    state: &BlockState<'tcx>,
    call: Option<&rustc_middle::mir::TerminatorKind<'tcx>>,
    return_lock_summaries: &HashMap<DefId, BTreeSet<LockInfoArtifact>>,
) -> Option<CallEffect<'tcx>> {
    let rustc_middle::mir::TerminatorKind::Call {
        func,
        args,
        destination,
        ..
    } = call?
    else {
        return None;
    };

    let def_id = place::resolve_called_def_id(body, func.clone())?;
    let item_name = tcx.item_name(def_id);
    let method_name = item_name.as_str();
    let callee_path = tcx.def_path_str(def_id);

    if method_name == "drop" {
        let local = args.first()?.node.place()?.as_local()?;
        return Some(CallEffect::Release { local });
    }

    let destination = destination.as_local()?;

    if let Some(effect) = classify_return_lock_call(
        tcx,
        current_def_id,
        body,
        state,
        def_id,
        args,
        destination,
        return_lock_summaries,
    ) {
        return Some(effect);
    }

    let receiver = place::resolve_alias(state, args.first()?.node.place()?);
    let receiver_ty = place::peel_refs(args.first()?.node.ty(body, tcx));

    if method_name == "disable_irq" || method_name == "deref" {
        trace_lock_call(method_name, &callee_path, receiver_ty, true);
        return Some(CallEffect::Alias {
            destination,
            source: receiver,
        });
    }

    let lock = classify_lock_call(
        tcx,
        current_def_id,
        body,
        state,
        receiver,
        receiver_ty,
        method_name,
    );
    trace_lock_call(method_name, &callee_path, receiver_ty, lock.is_some());
    let lock = lock?;
    if matches!(lock.acquire.as_str(), "try_lock" | "try_read" | "try_write") {
        return None;
    }

    Some(CallEffect::Acquire { destination, lock })
}

fn classify_return_lock_call<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_def_id: DefId,
    body: &Body<'tcx>,
    state: &BlockState<'tcx>,
    callee: DefId,
    args: &[Spanned<Operand<'tcx>>],
    destination: Local,
    return_lock_summaries: &HashMap<DefId, BTreeSet<LockInfoArtifact>>,
) -> Option<CallEffect<'tcx>> {
    let arg_bindings = place::collect_call_arg_bindings(tcx, current_def_id, body, state, args);
    let returned_lock = return_lock_summaries.get(&callee)?;
    if returned_lock.len() != 1 {
        return None;
    }
    let returned_lock = returned_lock.first()?;
    let lock = place::rebind_lock_info(returned_lock, &arg_bindings)?;
    Some(CallEffect::Acquire { destination, lock })
}

fn trace_lock_call(method_name: &str, callee_path: &str, receiver_ty: Ty<'_>, matched: bool) {
    if env::var_os("LOCKDEP_TRACE_CALLS").is_none() {
        return;
    }
    if matches!(
        method_name,
        "lock" | "read" | "write" | "try_lock" | "try_read" | "try_write" | "disable_irq"
    ) {
        eprintln!(
            "lockdep trace: method={method_name} matched={matched} callee={callee_path} receiver_ty={receiver_ty}",
        );
    }
}

fn classify_lock_call<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_def_id: DefId,
    body: &Body<'tcx>,
    state: &BlockState<'tcx>,
    receiver: Place<'tcx>,
    receiver_ty: Ty<'tcx>,
    method_name: &str,
) -> Option<LockInfoArtifact> {
    let lock_ty = place::peel_refs(receiver_ty);
    let ty::Adt(adt, args) = lock_ty.kind() else {
        return None;
    };
    let primitive = tcx.item_name(adt.did()).to_string();

    let acquire = match (&*primitive, method_name) {
        ("SpinLock", "lock" | "try_lock")
        | ("RwLock", "read" | "write" | "try_read" | "try_write")
        | ("Mutex", "lock" | "try_lock")
        | ("RwMutex", "read" | "write" | "try_read" | "try_write") => method_name.to_string(),
        _ => return None,
    };

    let guard_behavior = guard_behavior_for_call(tcx, *adt, args, method_name);
    Some(LockInfoArtifact {
        class: place::lock_class_key(tcx, current_def_id, body, state, receiver),
        primitive,
        acquire,
        guard_behavior,
    })
}

fn guard_behavior_for_call<'tcx>(
    tcx: TyCtxt<'tcx>,
    adt: AdtDef<'tcx>,
    args: GenericArgsRef<'tcx>,
    method_name: &str,
) -> String {
    let item_name = tcx.item_name(adt.did());
    let primitive = item_name.as_str();
    if primitive == "Mutex" || primitive == "RwMutex" {
        return "Sleeping".into();
    }

    let guardian = args
        .types()
        .nth(1)
        .map(|ty| place::short_type_name(tcx, place::peel_refs(ty)))
        .unwrap_or_else(|| "Unknown".into());

    if primitive == "RwLock" && guardian == "WriteIrqDisabled" && method_name == "read" {
        "PreemptDisabled".into()
    } else {
        guardian
    }
}

pub(super) fn apply_effect<'tcx>(state: &mut BlockState<'tcx>, effect: &CallEffect<'tcx>) {
    match effect {
        CallEffect::Alias {
            destination,
            source,
        } => {
            state
                .aliases
                .insert(*destination, place::resolve_alias(state, *source));
            state.guards.remove(destination);
            state.local_origins.remove(destination);
        }
        CallEffect::Acquire { destination, lock } => {
            state.guards.insert(*destination, lock.clone());
            state.aliases.remove(destination);
            state.local_origins.remove(destination);
        }
        CallEffect::Release { local } => {
            state.guards.remove(local);
            state.aliases.remove(local);
            state.irq_guard_locals.remove(local);
            state.local_origins.remove(local);
        }
    }

    context::refresh_irq_state(state);
}

pub(super) fn emit_effect(
    events: &mut Vec<LockEventArtifact>,
    edges: &mut Vec<LockEdgeArtifact>,
    usage_states: &mut BTreeMap<LockInfoArtifact, LockUsageStateArtifact>,
    entry_locks: &mut BTreeSet<LockInfoArtifact>,
    body: &Body<'_>,
    state: &BlockState<'_>,
    base_context: context::ContextKind,
    effect: CallEffect<'_>,
    location: Option<SourceLocation>,
) {
    match effect {
        CallEffect::Alias { .. } => {}
        CallEffect::Acquire { destination, lock } => {
            let irq_state = context::irq_state_for_acquire(state, &lock);
            let context_kind = irq_state.current_context();
            let context = context_kind.as_str().to_string();
            if state.guards.is_empty() {
                entry_locks.insert(lock.clone());
            }
            for held in state.guards.values() {
                if summary::is_deadlock_relevant_lock_pair(held, &lock) {
                    edges.push(LockEdgeArtifact {
                        context: context.clone(),
                        from: held.clone(),
                        to: lock.clone(),
                        location: location.clone(),
                    });
                }
            }

            summary::record_lock_usage(
                usage_states,
                &lock,
                &irq_state,
                LockUsageSiteArtifact {
                    context: context.clone(),
                    location: location.clone(),
                },
            );

            events.push(LockEventArtifact {
                kind: "acquire".into(),
                context,
                lock,
                guard_local: Some(place::format_local(body, destination)),
                location,
            });
        }
        CallEffect::Release { local } => {
            if let Some(lock) = state.guards.get(&local).cloned() {
                events.push(LockEventArtifact {
                    kind: "release".into(),
                    context: context::current_irq_state(state, base_context)
                        .current_context()
                        .as_str()
                        .to_string(),
                    lock,
                    guard_local: Some(place::format_local(body, local)),
                    location,
                });
            }
        }
    }
}

pub(super) fn collect_callsite<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_def_id: DefId,
    body: &Body<'tcx>,
    state: &BlockState<'tcx>,
    base_context: context::ContextKind,
    call: Option<&rustc_middle::mir::TerminatorKind<'tcx>>,
    location: Option<SourceLocation>,
) -> Option<CallSite> {
    let rustc_middle::mir::TerminatorKind::Call {
        func,
        args,
        destination,
        ..
    } = call?
    else {
        return None;
    };
    let target = match place::resolve_called_target(body, func.clone())? {
        place::CallableTarget::Direct(callee)
            if callee.is_local() && tcx.def_kind(callee).is_fn_like() =>
        {
            CallTarget::Direct(callee)
        }
        place::CallableTarget::Arg(index) => CallTarget::CallableArg(index),
        place::CallableTarget::Direct(_) => return None,
    };

    Some(CallSite {
        target,
        destination: destination.as_local(),
        arg_bindings: place::collect_call_arg_bindings(tcx, current_def_id, body, state, args),
        callable_arg_bindings: args
            .iter()
            .enumerate()
            .filter_map(|(index, argument)| {
                place::resolve_called_target(body, argument.node.clone())
                    .map(|target| (index + 1, target))
            })
            .collect(),
        held_locks: state.guards.values().cloned().collect(),
        context: context::current_irq_state(state, base_context).current_context(),
        irq_state: context::current_irq_state(state, base_context),
        location,
    })
}

fn join_many_states<'tcx>(states: &[BlockState<'tcx>]) -> Option<BlockState<'tcx>> {
    let mut states = states.iter();
    let first = states.next()?.clone();
    Some(states.fold(first, |acc, state| join_states(&acc, state)))
}

fn join_states<'tcx>(left: &BlockState<'tcx>, right: &BlockState<'tcx>) -> BlockState<'tcx> {
    let aliases = left
        .aliases
        .iter()
        .filter_map(|(local, left_place)| {
            right
                .aliases
                .get(local)
                .filter(|right_place| *right_place == left_place)
                .map(|_| (*local, *left_place))
        })
        .collect();
    let guards = left
        .guards
        .iter()
        .filter_map(|(local, left_lock)| {
            right
                .guards
                .get(local)
                .filter(|right_lock| *right_lock == left_lock)
                .map(|_| (*local, left_lock.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let irq_guard_locals = left
        .irq_guard_locals
        .intersection(&right.irq_guard_locals)
        .copied()
        .collect::<BTreeSet<_>>();
    let local_origins = left
        .local_origins
        .iter()
        .filter_map(|(local, left_origin)| {
            right
                .local_origins
                .get(local)
                .filter(|right_origin| *right_origin == left_origin)
                .map(|_| (*local, left_origin.clone()))
        })
        .collect();

    let irq_state = {
        let mut state = BlockState::with_context(left.irq_state.current_context());
        state.guards = guards.clone();
        state.irq_guard_locals = irq_guard_locals.clone();
        state.irq_state = context::IrqExecutionState::merge(&left.irq_state, &right.irq_state);
        context::refresh_irq_state(&mut state);
        state.irq_state
    };

    BlockState {
        aliases,
        guards,
        irq_guard_locals,
        irq_state,
        local_origins,
    }
}
