// SPDX-License-Identifier: MPL-2.0

use rustc_abi::FieldIdx;
use rustc_middle::{
    mir::{AggregateKind, Body, Local, Operand, Place, ProjectionElem, Rvalue},
    ty::{self, Ty, TyCtxt},
};

use super::*;
use crate::model::{LockRootKey, ProjectionKey, SourceLocation};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CallableTarget {
    Direct(DefId),
    Arg(usize),
}

pub(super) fn dedup_edges(edges: &mut Vec<LockEdgeArtifact>) {
    edges.sort_by(|left, right| {
        left.context
            .cmp(&right.context)
            .then(left.from.class.cmp(&right.from.class))
            .then(left.from.primitive.cmp(&right.from.primitive))
            .then(left.from.acquire.cmp(&right.from.acquire))
            .then(left.to.class.cmp(&right.to.class))
            .then(left.to.primitive.cmp(&right.to.primitive))
            .then(left.to.acquire.cmp(&right.to.acquire))
            .then(left.location.cmp(&right.location))
    });
    edges.dedup_by(|left, right| {
        left.context == right.context
            && left.from == right.from
            && left.to == right.to
            && left.location == right.location
    });
}

pub(super) fn release_local<'tcx>(state: &mut BlockState<'tcx>, local: Local, _reason: &str) {
    state.aliases.remove(&local);
    state.guards.remove(&local);
    state.irq_guard_locals.remove(&local);
    state.local_origins.remove(&local);
    context::refresh_irq_state(state);
}

pub(super) fn resolve_alias<'tcx>(state: &BlockState<'tcx>, place: Place<'tcx>) -> Place<'tcx> {
    let Some(local) = place.as_local() else {
        return place;
    };
    state.aliases.get(&local).copied().unwrap_or(place)
}

pub(super) fn peel_refs<'tcx>(mut ty: Ty<'tcx>) -> Ty<'tcx> {
    while let ty::Ref(_, inner, _) = ty.kind() {
        ty = *inner;
    }
    ty
}

pub(super) fn type_key<'tcx>(value_ty: Ty<'tcx>) -> String {
    format!("{}", peel_refs(value_ty))
}

pub(super) fn short_type_name<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> String {
    match ty.kind() {
        ty::Adt(def, _) => tcx.item_name(def.did()).to_string(),
        _ => ty.to_string(),
    }
}

pub(super) fn classify_operand_origin<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_def_id: DefId,
    body: &Body<'tcx>,
    state: &BlockState<'tcx>,
    operand: &Operand<'tcx>,
) -> LocalOriginKey {
    match operand {
        Operand::Copy(source) | Operand::Move(source) if source.as_local().is_some() => {
            LocalOriginKey::AliasOf(Box::new(place_root_key(
                tcx,
                current_def_id,
                body,
                state,
                source.local,
            )))
        }
        Operand::Constant(constant) => constant
            .check_static_ptr(tcx)
            .map(|def_id| {
                LocalOriginKey::AliasOf(Box::new(LockRootKey::Global {
                    def_path: tcx.def_path_str(def_id),
                }))
            })
            .unwrap_or(LocalOriginKey::Unknown),
        _ => LocalOriginKey::Unknown,
    }
}

pub(super) fn transfer_guard_move<'tcx>(
    state: &mut BlockState<'tcx>,
    destination: Local,
    operand: &Operand<'tcx>,
) {
    let Operand::Move(source) = operand else {
        return;
    };
    let Some(source) = source.as_local() else {
        return;
    };
    let Some(lock) = state.guards.remove(&source) else {
        if state.irq_guard_locals.remove(&source) {
            state.irq_guard_locals.insert(destination);
            context::refresh_irq_state(state);
        }
        return;
    };
    state.guards.insert(destination, lock);
    context::refresh_irq_state(state);
}

pub(super) fn collect_call_arg_bindings<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_def_id: DefId,
    body: &Body<'tcx>,
    state: &BlockState<'tcx>,
    args: &[Spanned<Operand<'tcx>>],
) -> BTreeMap<usize, LockClassKey> {
    args.iter()
        .enumerate()
        .filter_map(|(index, argument)| {
            bind_call_argument(tcx, current_def_id, body, state, &argument.node)
                .map(|class| (index + 1, class))
        })
        .collect()
}

fn bind_call_argument<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_def_id: DefId,
    body: &Body<'tcx>,
    state: &BlockState<'tcx>,
    operand: &Operand<'tcx>,
) -> Option<LockClassKey> {
    match operand {
        Operand::Copy(place) | Operand::Move(place) => Some(lock_class_key(
            tcx,
            current_def_id,
            body,
            state,
            resolve_alias(state, *place),
        )),
        Operand::Constant(constant) => constant.check_static_ptr(tcx).map(|def_id| LockClassKey {
            root: LockRootKey::Global {
                def_path: tcx.def_path_str(def_id),
            },
            projections: Vec::new(),
        }),
    }
}

pub(super) fn lock_class_key<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_def_id: DefId,
    body: &Body<'tcx>,
    state: &BlockState<'tcx>,
    place: Place<'tcx>,
) -> LockClassKey {
    let root = place_root_key(tcx, current_def_id, body, state, place.local);
    let mut projections = Vec::new();
    let mut current_ty = body.local_decls[place.local].ty;
    let mut downcast_variant = None;

    for projection in place.projection.iter() {
        match projection {
            ProjectionElem::Deref => {
                if let Some(pointee_ty) = deref_ty(current_ty) {
                    projections.push(ProjectionKey::Deref {
                        pointee_ty: type_key(pointee_ty),
                    });
                    current_ty = pointee_ty;
                } else {
                    projections.push(ProjectionKey::Deref {
                        pointee_ty: type_key(current_ty),
                    });
                }
                downcast_variant = None;
            }
            ProjectionElem::Field(field, field_ty) => {
                let owner_ty = peel_refs(current_ty);
                let (owner_ty_name, field_name) = field_identity(owner_ty, downcast_variant, field);
                projections.push(ProjectionKey::Field {
                    owner_ty: owner_ty_name,
                    field_name,
                    field_index: field.index(),
                });
                current_ty = field_ty;
                downcast_variant = None;
            }
            ProjectionElem::Downcast(_, variant) => {
                let enum_ty = peel_refs(current_ty);
                projections.push(ProjectionKey::Downcast {
                    enum_ty: type_key(enum_ty),
                    variant_name: variant_name_for_ty(enum_ty, variant.as_usize()),
                });
                downcast_variant = Some(variant.as_usize());
            }
            ProjectionElem::Index(local) => {
                projections.push(ProjectionKey::Index {
                    index_local: local.as_usize(),
                });
            }
            ProjectionElem::ConstantIndex {
                offset,
                min_length,
                from_end,
            } => projections.push(ProjectionKey::ConstantIndex {
                offset: offset as u64,
                min_length: min_length as u64,
                from_end,
            }),
            ProjectionElem::Subslice { from, to, from_end } => {
                projections.push(ProjectionKey::Subslice {
                    from: from as u64,
                    to: to as u64,
                    from_end,
                });
            }
            ProjectionElem::OpaqueCast(ty) | ProjectionElem::UnwrapUnsafeBinder(ty) => {
                projections.push(ProjectionKey::OpaqueCast { ty: type_key(ty) });
                current_ty = ty;
                downcast_variant = None;
            }
        }
    }

    LockClassKey { root, projections }
}

pub(super) fn place_root_key<'tcx>(
    tcx: TyCtxt<'tcx>,
    current_def_id: DefId,
    body: &Body<'tcx>,
    state: &BlockState<'tcx>,
    local: Local,
) -> LockRootKey {
    let local_ty = body.local_decls[local].ty;
    match local.as_usize() {
        0 => LockRootKey::ReturnValue {
            fn_def_path: tcx.def_path_str(current_def_id),
            ty: type_key(local_ty),
        },
        1 if has_self_parameter(tcx, current_def_id) => LockRootKey::ReceiverArg {
            method_def_path: tcx.def_path_str(current_def_id),
            self_ty: type_key(local_ty),
        },
        index if index <= body.arg_count => LockRootKey::FnArg {
            fn_def_path: tcx.def_path_str(current_def_id),
            index,
            ty: type_key(local_ty),
        },
        index => {
            if let rustc_middle::mir::ClearCrossCrate::Set(local_info) =
                &body.local_decls[local].local_info
                && let rustc_middle::mir::LocalInfo::StaticRef { def_id, .. } = &**local_info
            {
                return LockRootKey::Global {
                    def_path: tcx.def_path_str(*def_id),
                };
            }

            if let Some(origin_root) = state
                .local_origins
                .get(&local)
                .and_then(local_origin_root_key)
            {
                return origin_root;
            }

            LockRootKey::Local {
                fn_def_path: tcx.def_path_str(current_def_id),
                index,
                ty: type_key(local_ty),
                origin: state.local_origins.get(&local).cloned().unwrap_or_default(),
            }
        }
    }
}

fn local_origin_root_key(origin: &LocalOriginKey) -> Option<LockRootKey> {
    match origin {
        LocalOriginKey::AliasOf(base) | LocalOriginKey::RefOfPlace { base } => {
            Some((**base).clone())
        }
        LocalOriginKey::Unknown | LocalOriginKey::AggregateTemp { .. } => None,
    }
}

fn has_self_parameter<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId) -> bool {
    tcx.opt_associated_item(def_id)
        .is_some_and(|associated_item| associated_item.is_method())
}

pub(super) fn rebind_lock_info(
    lock: &LockInfoArtifact,
    arg_bindings: &BTreeMap<usize, LockClassKey>,
) -> Option<LockInfoArtifact> {
    Some(LockInfoArtifact {
        class: rebind_lock_class(&lock.class, arg_bindings)?,
        primitive: lock.primitive.clone(),
        acquire: lock.acquire.clone(),
        guard_behavior: lock.guard_behavior.clone(),
    })
}

fn rebind_lock_class(
    lock_class: &LockClassKey,
    arg_bindings: &BTreeMap<usize, LockClassKey>,
) -> Option<LockClassKey> {
    let replacement = match &lock_class.root {
        LockRootKey::ReceiverArg { .. } => Some(arg_bindings.get(&1)?.clone()),
        LockRootKey::FnArg { index, .. } => Some(arg_bindings.get(index)?.clone()),
        _ => None,
    };

    if let Some(mut replacement) = replacement {
        replacement.projections.extend(
            lock_class
                .projections
                .iter()
                .skip(usize::from(has_redundant_boundary_deref(
                    &replacement.projections,
                    &lock_class.projections,
                )))
                .cloned(),
        );
        Some(replacement)
    } else {
        Some(lock_class.clone())
    }
}

fn has_redundant_boundary_deref(left: &[ProjectionKey], right: &[ProjectionKey]) -> bool {
    matches!(
        (left.last(), right.first()),
        (
            Some(ProjectionKey::Deref { .. }),
            Some(ProjectionKey::Deref { .. })
        )
    )
}

fn deref_ty<'tcx>(ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    ty.builtin_deref(true)
}

fn field_identity<'tcx>(
    owner_ty: Ty<'tcx>,
    downcast_variant: Option<usize>,
    field: FieldIdx,
) -> (String, String) {
    match owner_ty.kind() {
        ty::Adt(adt, _) => {
            let owner_ty_name = type_key(owner_ty);
            let field_name = if adt.is_enum() {
                downcast_variant
                    .and_then(|variant_index| adt.variants().iter().nth(variant_index))
                    .and_then(|variant| variant.fields.get(field))
                    .map(|field| field.name.to_string())
            } else {
                adt.all_fields()
                    .nth(field.index())
                    .map(|field| field.name.to_string())
            }
            .unwrap_or_else(|| format!("field{}", field.index()));
            (owner_ty_name, field_name)
        }
        ty::Tuple(_) => (type_key(owner_ty), format!("tuple_field{}", field.index())),
        _ => (type_key(owner_ty), format!("field{}", field.index())),
    }
}

fn variant_name_for_ty<'tcx>(enum_ty: Ty<'tcx>, variant_index: usize) -> String {
    match enum_ty.kind() {
        ty::Adt(adt, _) if adt.is_enum() => adt
            .variants()
            .iter()
            .nth(variant_index)
            .map(|variant| variant.name.to_string())
            .unwrap_or_else(|| format!("variant{variant_index}")),
        _ => format!("variant{variant_index}"),
    }
}

pub(super) fn span_to_location<'tcx>(
    tcx: TyCtxt<'tcx>,
    span: rustc_span::Span,
) -> Option<SourceLocation> {
    if span.is_dummy() {
        return None;
    }
    let source_map = tcx.sess.source_map();
    let location = source_map.lookup_char_pos(span.lo());
    Some(SourceLocation {
        file: location.file.name.prefer_local().to_string(),
        line: location.line,
        column: location.col_display + 1,
    })
}

pub(super) fn collect_callable_locals<'tcx>(body: &Body<'tcx>) -> BTreeMap<Local, CallableTarget> {
    let mut callable_locals = BTreeMap::new();

    for (local, local_decl) in body.local_decls.iter_enumerated() {
        if let rustc_middle::mir::ClearCrossCrate::Set(local_info) = &local_decl.local_info
            && let rustc_middle::mir::LocalInfo::StaticRef { def_id, .. } = &**local_info
        {
            callable_locals.insert(local, CallableTarget::Direct(*def_id));
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        for block_data in body.basic_blocks.iter() {
            for statement in &block_data.statements {
                let Some((place, rvalue)) = statement.kind.as_assign() else {
                    continue;
                };
                let Some(local) = place.as_local() else {
                    continue;
                };
                let resolved = match rvalue {
                    Rvalue::Aggregate(aggregate_kind, _) => match &**aggregate_kind {
                        AggregateKind::Closure(def_id, _)
                        | AggregateKind::CoroutineClosure(def_id, _) => {
                            Some(CallableTarget::Direct(*def_id))
                        }
                        _ => None,
                    },
                    Rvalue::Use(operand) | Rvalue::Cast(_, operand, _) => operand
                        .const_fn_def()
                        .map(|(def_id, _)| CallableTarget::Direct(def_id))
                        .or_else(|| {
                            operand
                                .place()
                                .and_then(|place| place.as_local())
                                .and_then(|source| {
                                    callable_locals.get(&source).copied().or_else(|| {
                                        callable_arg_index(body, source).map(CallableTarget::Arg)
                                    })
                                })
                        }),
                    _ => None,
                };
                if let Some(target) = resolved
                    && callable_locals.get(&local) != Some(&target)
                {
                    callable_locals.insert(local, target);
                    changed = true;
                }
            }
        }
    }

    callable_locals
}

pub(super) fn resolve_callback_operand<'tcx>(
    operand: Operand<'tcx>,
    callable_locals: &BTreeMap<Local, CallableTarget>,
) -> Option<DefId> {
    if let Some((def_id, _)) = operand.const_fn_def() {
        return Some(def_id);
    }

    let (Operand::Move(place) | Operand::Copy(place)) = operand else {
        return None;
    };
    let local = place.as_local()?;
    match callable_locals.get(&local).copied()? {
        CallableTarget::Direct(def_id) => Some(def_id),
        CallableTarget::Arg(_) => None,
    }
}

pub(super) fn resolve_called_target<'tcx>(
    body: &Body<'tcx>,
    operand: Operand<'tcx>,
) -> Option<CallableTarget> {
    let callable_locals = collect_callable_locals(body);
    if let Some((def_id, _)) = operand.const_fn_def() {
        return Some(CallableTarget::Direct(def_id));
    }

    let (Operand::Move(place) | Operand::Copy(place)) = operand else {
        return None;
    };
    let local = place.as_local()?;
    callable_locals
        .get(&local)
        .copied()
        .or_else(|| callable_arg_index(body, local).map(CallableTarget::Arg))
}

pub(super) fn resolve_called_def_id<'tcx>(
    body: &Body<'tcx>,
    operand: Operand<'tcx>,
) -> Option<DefId> {
    match resolve_called_target(body, operand)? {
        CallableTarget::Direct(def_id) => Some(def_id),
        CallableTarget::Arg(_) => None,
    }
}

pub(super) fn format_local<'tcx>(body: &Body<'tcx>, local: Local) -> String {
    if local.as_usize() == 0 {
        return "return".into();
    }
    if local.as_usize() <= body.arg_count {
        return format!("arg{}", local.as_usize());
    }
    format!("_{}", local.as_usize())
}

fn callable_arg_index<'tcx>(body: &Body<'tcx>, local: Local) -> Option<usize> {
    let index = local.as_usize();
    (index != 0
        && index <= body.arg_count
        && is_callable_type(peel_refs(body.local_decls[local].ty)))
    .then_some(index)
}

fn is_callable_type<'tcx>(ty: Ty<'tcx>) -> bool {
    matches!(
        ty.kind(),
        ty::FnDef(..) | ty::FnPtr(..) | ty::Closure(..) | ty::CoroutineClosure(..)
    )
}
