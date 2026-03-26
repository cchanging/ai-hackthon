// SPDX-License-Identifier: MPL-2.0

use rustc_middle::mir::{Body, Local, TerminatorKind};

use super::*;

/// Execution contexts currently recognized by the prototype.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ContextKind {
    Task,
    TaskIrqDisabled,
    HardIrqTopHalf,
    BottomHalfL1,
    BottomHalfL1IrqDisabled,
    BottomHalfL2,
}

/// Structured IRQ execution state tracked during MIR analysis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct IrqExecutionState {
    base_context: ContextKind,
    base_hardirq_enabled: bool,
    base_softirq_enabled: bool,
    pub(super) hardirq_context_depth: u8,
    pub(super) softirq_context_depth: u8,
    pub(super) hardirq_enabled: bool,
    pub(super) softirq_enabled: bool,
}

impl IrqExecutionState {
    pub(super) fn from_context(context: ContextKind) -> Self {
        match context {
            ContextKind::Task => Self {
                base_context: ContextKind::Task,
                base_hardirq_enabled: true,
                base_softirq_enabled: true,
                hardirq_context_depth: 0,
                softirq_context_depth: 0,
                hardirq_enabled: true,
                softirq_enabled: true,
            },
            ContextKind::TaskIrqDisabled => Self {
                base_context: ContextKind::Task,
                base_hardirq_enabled: false,
                base_softirq_enabled: false,
                hardirq_context_depth: 0,
                softirq_context_depth: 0,
                hardirq_enabled: false,
                softirq_enabled: false,
            },
            ContextKind::HardIrqTopHalf => Self {
                base_context: ContextKind::HardIrqTopHalf,
                base_hardirq_enabled: false,
                base_softirq_enabled: false,
                hardirq_context_depth: 1,
                softirq_context_depth: 0,
                hardirq_enabled: false,
                softirq_enabled: false,
            },
            ContextKind::BottomHalfL1 => Self {
                base_context: ContextKind::BottomHalfL1,
                base_hardirq_enabled: true,
                base_softirq_enabled: false,
                hardirq_context_depth: 0,
                softirq_context_depth: 1,
                hardirq_enabled: true,
                softirq_enabled: false,
            },
            ContextKind::BottomHalfL1IrqDisabled => Self {
                base_context: ContextKind::BottomHalfL1,
                base_hardirq_enabled: false,
                base_softirq_enabled: false,
                hardirq_context_depth: 0,
                softirq_context_depth: 1,
                hardirq_enabled: false,
                softirq_enabled: false,
            },
            ContextKind::BottomHalfL2 => Self {
                base_context: ContextKind::BottomHalfL2,
                base_hardirq_enabled: false,
                base_softirq_enabled: false,
                hardirq_context_depth: 2,
                softirq_context_depth: 0,
                hardirq_enabled: false,
                softirq_enabled: false,
            },
        }
    }

    pub(super) fn current_context(&self) -> ContextKind {
        match (self.base_context, self.hardirq_enabled) {
            (ContextKind::Task, false) => ContextKind::TaskIrqDisabled,
            (ContextKind::BottomHalfL1, false) => ContextKind::BottomHalfL1IrqDisabled,
            _ => self.base_context,
        }
    }

    pub(super) fn for_acquire(&self, lock: &LockInfoArtifact) -> Self {
        let mut next = self.clone();
        if lock_disables_irq_while_held(lock) {
            next.hardirq_enabled = false;
            next.softirq_enabled = false;
        }
        next
    }

    pub(super) fn merge(left: &Self, right: &Self) -> Self {
        let mut merged = left.clone();
        merged.hardirq_context_depth = left.hardirq_context_depth.max(right.hardirq_context_depth);
        merged.softirq_context_depth = left.softirq_context_depth.max(right.softirq_context_depth);
        merged.hardirq_enabled = left.hardirq_enabled || right.hardirq_enabled;
        merged.softirq_enabled = left.softirq_enabled || right.softirq_enabled;
        merged.base_hardirq_enabled = left.base_hardirq_enabled || right.base_hardirq_enabled;
        merged.base_softirq_enabled = left.base_softirq_enabled || right.base_softirq_enabled;
        merged
    }
}

pub(super) fn softirq_entry_state_for_body<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &Body<'tcx>,
    base_context: ContextKind,
) -> BlockState<'tcx> {
    let mut entry_state = BlockState::with_context(base_context);
    if base_context == ContextKind::BottomHalfL1
        && body.arg_count >= 1
        && place::short_type_name(
            tcx,
            place::peel_refs(body.local_decls[Local::from_usize(1)].ty),
        ) == "DisabledLocalIrqGuard"
    {
        entry_state.irq_guard_locals.insert(Local::from_usize(1));
        refresh_irq_state(&mut entry_state);
    }
    entry_state
}

pub(super) fn current_context(state: &BlockState<'_>, base_context: ContextKind) -> ContextKind {
    let _ = base_context;
    state.irq_state.current_context()
}

pub(super) fn current_irq_state(
    state: &BlockState<'_>,
    base_context: ContextKind,
) -> IrqExecutionState {
    let _ = base_context;
    state.irq_state.clone()
}

pub(super) fn irq_state_for_acquire(
    state: &BlockState<'_>,
    lock: &LockInfoArtifact,
) -> IrqExecutionState {
    state.irq_state.for_acquire(lock)
}

pub(super) fn refresh_irq_state(state: &mut BlockState<'_>) {
    let mut irq_state = state.irq_state.clone();
    irq_state.hardirq_enabled = irq_state.base_hardirq_enabled;
    irq_state.softirq_enabled = irq_state.base_softirq_enabled;
    if !state.irq_guard_locals.is_empty() || state.guards.values().any(lock_disables_irq_while_held)
    {
        irq_state.hardirq_enabled = false;
        irq_state.softirq_enabled = false;
    }
    state.irq_state = irq_state;
}

pub(super) fn lock_disables_irq_while_held(lock: &LockInfoArtifact) -> bool {
    lock.guard_behavior == "LocalIrqDisabled"
        || (lock.guard_behavior == "WriteIrqDisabled" && lock.acquire == "write")
}

impl ContextKind {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Task => "Task",
            Self::TaskIrqDisabled => "TaskIrqDisabled",
            Self::HardIrqTopHalf => "HardIrqTopHalf",
            Self::BottomHalfL1 => "BottomHalfL1",
            Self::BottomHalfL1IrqDisabled => "BottomHalfL1IrqDisabled",
            Self::BottomHalfL2 => "BottomHalfL2",
        }
    }
}

pub(super) fn collect_entry_contexts<'tcx>(tcx: TyCtxt<'tcx>) -> HashMap<DefId, Vec<ContextKind>> {
    let mut map: HashMap<DefId, BTreeSet<ContextKind>> = HashMap::new();
    let configured_entries = load_configured_irq_entries();

    for local_def_id in tcx.mir_keys(()) {
        let def_id = local_def_id.to_def_id();
        if !tcx.def_kind(def_id).is_fn_like() {
            continue;
        }
        if tcx.visibility(def_id).is_public() {
            map.entry(def_id).or_default().insert(ContextKind::Task);
        }
        let body = tcx.optimized_mir(def_id);
        let callable_locals = place::collect_callable_locals(body);

        for block_data in body.basic_blocks.iter() {
            let Some(terminator) = &block_data.terminator else {
                continue;
            };
            let TerminatorKind::Call { func, args, .. } = &terminator.kind else {
                continue;
            };
            let Some((callee_def_id, _)) = func.const_fn_def() else {
                continue;
            };
            let callee_path = tcx.def_path_str(callee_def_id);
            let Some((context, callback_arg_index)) =
                builtin_irq_entry_descriptor(tcx, callee_def_id, &callee_path)
                    .or_else(|| configured_irq_entry_descriptor(&callee_path, &configured_entries))
            else {
                continue;
            };

            let Some(callback) = args.get(callback_arg_index) else {
                continue;
            };
            if let Some(target_def_id) =
                place::resolve_callback_operand(callback.node.clone(), &callable_locals)
            {
                map.entry(target_def_id).or_default().insert(context);
            }
        }
    }

    map.into_iter()
        .map(|(def_id, contexts)| (def_id, contexts.into_iter().collect()))
        .collect()
}

fn load_configured_irq_entries() -> Vec<ConfiguredIrqEntry> {
    env::var("LOCKDEP_IRQ_ENTRIES_JSON")
        .ok()
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

fn builtin_irq_entry_descriptor<'tcx>(
    tcx: TyCtxt<'tcx>,
    callee_def_id: DefId,
    _callee_path: &str,
) -> Option<(ContextKind, usize)> {
    let item_name = tcx.item_name(callee_def_id).as_str().to_string();
    match item_name.as_str() {
        "register_bottom_half_handler_l1" => Some((ContextKind::BottomHalfL1, 0)),
        "register_bottom_half_handler_l2" => Some((ContextKind::BottomHalfL2, 0)),
        "on_active" => Some((ContextKind::HardIrqTopHalf, 1)),
        _ => None,
    }
}

fn configured_irq_entry_descriptor(
    callee_path: &str,
    configured_entries: &[ConfiguredIrqEntry],
) -> Option<(ContextKind, usize)> {
    configured_entries.iter().find_map(|entry| {
        callee_path.ends_with(&entry.function).then(|| {
            parse_context_kind(&entry.context).map(|context| (context, entry.callback_arg_index))
        })?
    })
}

fn parse_context_kind(text: &str) -> Option<ContextKind> {
    match text {
        "Task" => Some(ContextKind::Task),
        "TaskIrqDisabled" => Some(ContextKind::TaskIrqDisabled),
        "HardIrqTopHalf" => Some(ContextKind::HardIrqTopHalf),
        "BottomHalfL1" => Some(ContextKind::BottomHalfL1),
        "BottomHalfL1IrqDisabled" => Some(ContextKind::BottomHalfL1IrqDisabled),
        "BottomHalfL2" => Some(ContextKind::BottomHalfL2),
        _ => None,
    }
}
