use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::Span;
use syn::spanned::Spanned;
use syn::visit::Visit;

use crate::error::AppError;
use crate::model::{
    CrateView, EnumVariantView, FnParamView, FnSignatureView, FnTypeView, GraphEdge, GraphEdgeRef,
    GraphIndex, GraphNode, ImplDetail, ItemView, MethodView, ModuleView, RustMapOutput, SourceSpan,
    StructFieldView, TraitMethodView, VariantFieldView, WarningItem, WorkspaceInfo,
};
use crate::workspace::WorkspaceSnapshot;

#[derive(Debug)]
struct NodeBuilder {
    item: ItemView,
    method_index: BTreeMap<String, MethodView>,
    trait_methods: BTreeMap<String, TraitMethodView>,
    impl_details: BTreeMap<String, String>,
    call_specs: Vec<CallSpec>,
}

#[derive(Debug, Clone)]
struct UseSpec {
    raw_path: String,
    alias: Option<String>,
}

#[derive(Debug, Default, Clone)]
struct ModuleState {
    file: Option<String>,
    node_ids: Vec<String>,
    uses: Vec<UseSpec>,
}

#[derive(Debug, Clone)]
struct TraitInheritPending {
    source_trait_id: String,
    target_raw_path: String,
    module_path: String,
    crate_name: String,
    location: String,
}

#[derive(Debug, Clone)]
struct ImplPending {
    self_ty_raw: String,
    trait_raw: Option<String>,
    methods: Vec<ImplMethodPending>,
    module_path: String,
    crate_name: String,
    location: String,
}

#[derive(Debug, Clone)]
struct ImplMethodPending {
    name: String,
    signature: FnSignatureView,
    calls: Vec<CallSpec>,
    file: String,
    span: SourceSpan,
}

#[derive(Debug, Clone)]
struct CallSpec {
    target: CallTarget,
    location: String,
}

#[derive(Debug, Clone)]
enum CallTarget {
    Path(String),
    QualifiedPath {
        self_ty_raw: String,
        trait_raw: Option<String>,
        method: String,
    },
    Method {
        method: String,
        receiver: ReceiverHint,
    },
}

#[derive(Debug, Clone)]
enum ReceiverHint {
    SelfValue,
    SelfType,
    Path(String),
    Ident(String),
    Unknown,
}

#[derive(Debug)]
struct MethodLookup {
    by_owner_inherent: HashMap<(String, String), String>,
    by_owner_trait: HashMap<(String, String, String), String>,
    by_owner_name: HashMap<(String, String), Vec<String>>,
    callable_ids: HashSet<String>,
}

pub fn extract_workspace(snapshot: &WorkspaceSnapshot) -> Result<RustMapOutput, AppError> {
    let workspace_member_names = snapshot
        .members
        .iter()
        .map(|m| m.name.clone())
        .collect::<HashSet<_>>();

    let mut nodes: BTreeMap<String, NodeBuilder> = BTreeMap::new();
    let mut modules: BTreeMap<String, ModuleState> = BTreeMap::new();
    let mut trait_inherit_pending: Vec<TraitInheritPending> = Vec::new();
    let mut impl_pending: Vec<ImplPending> = Vec::new();
    let mut warnings: Vec<WarningItem> = Vec::new();

    for member in &snapshot.members {
        if member.rust_files.is_empty() {
            continue;
        }
        for file in &member.rust_files {
            let src = fs::read_to_string(file).map_err(|source| AppError::Io {
                path: file.clone(),
                source,
            })?;
            validate_with_rust_analyzer(file, &src)?;

            let parsed = syn::parse_file(&src).map_err(|err| AppError::ParseFile {
                path: file.clone(),
                message: err.to_string(),
            })?;
            let source_root = source_root_for_file(member, file)?;
            let module_segments = module_segments_from_file(source_root, file)?;
            visit_items(
                &member.name,
                &module_segments,
                file,
                &parsed.items,
                &mut nodes,
                &mut modules,
                &mut trait_inherit_pending,
                &mut impl_pending,
                &mut warnings,
            );
        }
    }

    let (base_symbol_ids, base_name_index) = build_symbol_index(&nodes);

    let mut edges = HashSet::<(String, String, String, String)>::new();

    let mut module_imports: HashMap<String, HashMap<String, String>> = HashMap::new();
    for (module_path, module_state) in &modules {
        let crate_name = crate_from_module_path(module_path);
        let imports = module_imports.entry(module_path.clone()).or_default();
        for use_spec in &module_state.uses {
            if let Some(target_id) = resolve_symbol(
                &use_spec.raw_path,
                module_path,
                &crate_name,
                imports,
                &base_symbol_ids,
                &base_name_index,
                &workspace_member_names,
            ) {
                let alias = use_spec
                    .alias
                    .clone()
                    .unwrap_or_else(|| last_segment(&use_spec.raw_path).to_string());
                imports.insert(alias, target_id);
            }
        }
    }

    for pending in trait_inherit_pending {
        let imports = module_imports
            .get(&pending.module_path)
            .cloned()
            .unwrap_or_default();
        match resolve_symbol(
            &pending.target_raw_path,
            &pending.module_path,
            &pending.crate_name,
            &imports,
            &base_symbol_ids,
            &base_name_index,
            &workspace_member_names,
        ) {
            Some(target_id) => {
                edges.insert((
                    "inherit".to_string(),
                    pending.source_trait_id.clone(),
                    target_id,
                    "supertrait".to_string(),
                ));
            }
            None => {
                if should_warn_unresolved_path(
                    &pending.target_raw_path,
                    &imports,
                    &base_name_index,
                    &workspace_member_names,
                ) {
                    warnings.push(warning(
                        "unresolved_inherit",
                        format!("cannot resolve supertrait `{}`", pending.target_raw_path),
                        Some(pending.location),
                    ));
                }
            }
        }
    }

    for pending in impl_pending {
        let imports = module_imports
            .get(&pending.module_path)
            .cloned()
            .unwrap_or_default();
        let self_id = resolve_symbol(
            &pending.self_ty_raw,
            &pending.module_path,
            &pending.crate_name,
            &imports,
            &base_symbol_ids,
            &base_name_index,
            &workspace_member_names,
        );
        let Some(self_id) = self_id else {
            warnings.push(warning(
                "unresolved_impl_target",
                format!("cannot resolve impl target `{}`", pending.self_ty_raw),
                Some(pending.location),
            ));
            continue;
        };

        let resolved_trait_id = if let Some(trait_raw) = &pending.trait_raw {
            match resolve_symbol(
                trait_raw,
                &pending.module_path,
                &pending.crate_name,
                &imports,
                &base_symbol_ids,
                &base_name_index,
                &workspace_member_names,
            ) {
                Some(trait_id) => {
                    if let Some(node) = nodes.get_mut(&self_id) {
                        node.impl_details
                            .insert(trait_id.clone(), trait_raw.clone());
                    }
                    edges.insert((
                        "impl".to_string(),
                        self_id.clone(),
                        trait_id.clone(),
                        "trait_impl".to_string(),
                    ));
                    Some(trait_id)
                }
                None => {
                    if should_warn_unresolved_path(
                        trait_raw,
                        &imports,
                        &base_name_index,
                        &workspace_member_names,
                    ) {
                        warnings.push(warning(
                            "unresolved_impl_trait",
                            format!("cannot resolve trait `{trait_raw}` in impl block"),
                            Some(pending.location.clone()),
                        ));
                    }
                    None
                }
            }
        } else {
            None
        };

        let owner_kind = nodes
            .get(&self_id)
            .map(|node| node.item.kind.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let method_source = if resolved_trait_id.is_some() {
            "trait"
        } else {
            "inherent"
        };

        for method in pending.methods {
            let method_id = method_item_id(
                &self_id,
                &method.name,
                method_source,
                resolved_trait_id.as_deref(),
            );
            if !nodes.contains_key(&method_id) {
                let mut method_item = make_item_view_from_source(
                    "method",
                    &pending.module_path,
                    &method.name,
                    &method.file,
                    method.span.clone(),
                );
                method_item.id = method_id.clone();
                method_item.fn_signature = Some(method.signature.clone());
                method_item.owner_id = Some(self_id.clone());
                method_item.owner_kind = Some(owner_kind.clone());
                method_item.source = Some(method_source.to_string());
                method_item.trait_id = resolved_trait_id.clone();
                nodes.insert(
                    method_id.clone(),
                    NodeBuilder {
                        item: method_item,
                        method_index: BTreeMap::new(),
                        trait_methods: BTreeMap::new(),
                        impl_details: BTreeMap::new(),
                        call_specs: method.calls.clone(),
                    },
                );
                modules
                    .entry(pending.module_path.clone())
                    .or_default()
                    .node_ids
                    .push(method_id.clone());
            }

            if let Some(owner_node) = nodes.get_mut(&self_id) {
                let method_key =
                    method_index_key(&method.name, method_source, resolved_trait_id.as_deref());
                owner_node.method_index.insert(
                    method_key,
                    MethodView {
                        name: method.name,
                        source: method_source.to_string(),
                        trait_id: resolved_trait_id.clone(),
                        method_id: Some(method_id),
                    },
                );
            }
        }
    }

    let (symbol_ids, name_index) = build_symbol_index(&nodes);
    resolve_item_types(
        &mut nodes,
        &module_imports,
        &symbol_ids,
        &name_index,
        &workspace_member_names,
    );
    add_contain_edges(&nodes, &mut edges);

    let method_lookup = build_method_lookup(&nodes);
    resolve_call_edges(
        &nodes,
        &module_imports,
        &symbol_ids,
        &name_index,
        &workspace_member_names,
        &method_lookup,
        &mut edges,
        &mut warnings,
    );

    let finalized_items = finalize_items(nodes);
    let graph_index = build_graph_index(&finalized_items, edges);
    let crates = build_crate_views(&snapshot.members, &modules, &finalized_items);

    Ok(RustMapOutput {
        workspace: WorkspaceInfo {
            root: snapshot.root.display().to_string(),
            members: snapshot.members.iter().map(|m| m.name.clone()).collect(),
        },
        dependencies: snapshot.dependencies.clone(),
        crates,
        graph_index,
        warnings,
    })
}

fn validate_with_rust_analyzer(path: &Path, source: &str) -> Result<(), AppError> {
    let parse = ra_ap_syntax::SourceFile::parse(source, ra_ap_syntax::Edition::CURRENT);
    let errors = parse.errors();
    if errors.is_empty() {
        return Ok(());
    }
    let msg = errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ");
    Err(AppError::ParseFile {
        path: path.to_path_buf(),
        message: format!("rust-analyzer parse errors: {msg}"),
    })
}

#[allow(clippy::too_many_arguments)]
fn visit_items(
    crate_name: &str,
    module_segments: &[String],
    file_path: &Path,
    items: &[syn::Item],
    nodes: &mut BTreeMap<String, NodeBuilder>,
    modules: &mut BTreeMap<String, ModuleState>,
    trait_inherit_pending: &mut Vec<TraitInheritPending>,
    impl_pending: &mut Vec<ImplPending>,
    warnings: &mut Vec<WarningItem>,
) {
    let module_path = to_module_path(crate_name, module_segments);
    let module_state = modules.entry(module_path.clone()).or_default();
    if module_state.file.is_none() {
        module_state.file = Some(file_path.display().to_string());
    }

    for item in items {
        match item {
            syn::Item::Struct(s) => {
                let mut item_view = make_item_view(
                    "struct",
                    &module_path,
                    &s.ident.to_string(),
                    file_path,
                    s.span(),
                );
                let (struct_shape, struct_fields) = struct_fields_from_syn(&s.fields);
                item_view.struct_shape = Some(struct_shape);
                item_view.struct_fields = struct_fields;
                modules
                    .entry(module_path.clone())
                    .or_default()
                    .node_ids
                    .push(item_view.id.clone());
                nodes.insert(item_view.id.clone(), new_node_builder(item_view));
            }
            syn::Item::Enum(e) => {
                let mut item_view = make_item_view(
                    "enum",
                    &module_path,
                    &e.ident.to_string(),
                    file_path,
                    e.span(),
                );
                item_view.enum_variants = enum_variants_from_syn(e);
                modules
                    .entry(module_path.clone())
                    .or_default()
                    .node_ids
                    .push(item_view.id.clone());
                nodes.insert(item_view.id.clone(), new_node_builder(item_view));
            }
            syn::Item::Trait(t) => {
                let mut item_view = make_item_view(
                    "trait",
                    &module_path,
                    &t.ident.to_string(),
                    file_path,
                    t.span(),
                );
                item_view.trait_methods = t
                    .items
                    .iter()
                    .filter_map(|item| match item {
                        syn::TraitItem::Fn(method) => Some(trait_method_view_from_syn(method)),
                        _ => None,
                    })
                    .collect();
                let node_id = item_view.id.clone();
                modules
                    .entry(module_path.clone())
                    .or_default()
                    .node_ids
                    .push(node_id.clone());
                nodes.insert(node_id.clone(), new_node_builder(item_view));

                for bound in &t.supertraits {
                    if let syn::TypeParamBound::Trait(tb) = bound {
                        trait_inherit_pending.push(TraitInheritPending {
                            source_trait_id: node_id.clone(),
                            target_raw_path: path_to_string(&tb.path),
                            module_path: module_path.clone(),
                            crate_name: crate_name.to_string(),
                            location: format!(
                                "{}:{}",
                                file_path.display(),
                                tb.path.span().start().line
                            ),
                        });
                    }
                }
            }
            syn::Item::Fn(f) => {
                let mut item_view = make_item_view(
                    "fn",
                    &module_path,
                    &f.sig.ident.to_string(),
                    file_path,
                    f.span(),
                );
                item_view.fn_signature = Some(fn_signature_from_syn(f));
                let call_specs = collect_call_specs_from_block(&f.block, file_path);
                modules
                    .entry(module_path.clone())
                    .or_default()
                    .node_ids
                    .push(item_view.id.clone());
                let mut node = new_node_builder(item_view);
                node.call_specs = call_specs;
                nodes.insert(node.item.id.clone(), node);
            }
            syn::Item::Type(t) => {
                let item_view = make_item_view(
                    "type",
                    &module_path,
                    &t.ident.to_string(),
                    file_path,
                    t.span(),
                );
                modules
                    .entry(module_path.clone())
                    .or_default()
                    .node_ids
                    .push(item_view.id.clone());
                nodes.insert(item_view.id.clone(), new_node_builder(item_view));
            }
            syn::Item::Const(c) => {
                let item_view = make_item_view(
                    "const",
                    &module_path,
                    &c.ident.to_string(),
                    file_path,
                    c.span(),
                );
                modules
                    .entry(module_path.clone())
                    .or_default()
                    .node_ids
                    .push(item_view.id.clone());
                nodes.insert(item_view.id.clone(), new_node_builder(item_view));
            }
            syn::Item::Static(s) => {
                let item_view = make_item_view(
                    "static",
                    &module_path,
                    &s.ident.to_string(),
                    file_path,
                    s.span(),
                );
                modules
                    .entry(module_path.clone())
                    .or_default()
                    .node_ids
                    .push(item_view.id.clone());
                nodes.insert(item_view.id.clone(), new_node_builder(item_view));
            }
            syn::Item::Use(u) => {
                let mut entries = Vec::new();
                collect_use_specs(&u.tree, String::new(), &mut entries);
                for (raw_path, alias) in entries {
                    modules
                        .entry(module_path.clone())
                        .or_default()
                        .uses
                        .push(UseSpec { raw_path, alias });
                }
            }
            syn::Item::Impl(i) => {
                let Some(self_ty_raw) = type_to_string(i.self_ty.as_ref()) else {
                    warnings.push(warning(
                        "unsupported_impl_type",
                        "unsupported impl self type, skipping impl block".to_string(),
                        Some(format!("{}:{}", file_path.display(), i.span().start().line)),
                    ));
                    continue;
                };
                let trait_raw = i
                    .trait_
                    .as_ref()
                    .map(|(_, path, _)| path_to_string(path))
                    .filter(|s| !s.is_empty());
                let methods = i
                    .items
                    .iter()
                    .filter_map(|it| match it {
                        syn::ImplItem::Fn(f) => Some(ImplMethodPending {
                            name: f.sig.ident.to_string(),
                            signature: fn_signature_from_sig(&f.sig),
                            calls: collect_call_specs_from_block(&f.block, file_path),
                            file: file_path.display().to_string(),
                            span: span_to_source(f.span()),
                        }),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                impl_pending.push(ImplPending {
                    self_ty_raw,
                    trait_raw,
                    methods,
                    module_path: module_path.clone(),
                    crate_name: crate_name.to_string(),
                    location: format!("{}:{}", file_path.display(), i.span().start().line),
                });
            }
            syn::Item::Mod(m) => {
                let mut nested_segments = module_segments.to_vec();
                nested_segments.push(m.ident.to_string());
                let nested_path = to_module_path(crate_name, &nested_segments);
                modules.entry(nested_path.clone()).or_default().file =
                    Some(file_path.display().to_string());

                if let Some((_, nested_items)) = &m.content {
                    visit_items(
                        crate_name,
                        &nested_segments,
                        file_path,
                        nested_items,
                        nodes,
                        modules,
                        trait_inherit_pending,
                        impl_pending,
                        warnings,
                    );
                }
            }
            _ => {}
        }
    }
}

fn new_node_builder(item: ItemView) -> NodeBuilder {
    NodeBuilder {
        item,
        method_index: BTreeMap::new(),
        trait_methods: BTreeMap::new(),
        impl_details: BTreeMap::new(),
        call_specs: Vec::new(),
    }
}

fn make_item_view(
    kind: &str,
    module_path: &str,
    name: &str,
    file_path: &Path,
    span: Span,
) -> ItemView {
    make_item_view_from_source(
        kind,
        module_path,
        name,
        &file_path.display().to_string(),
        span_to_source(span),
    )
}

fn make_item_view_from_source(
    kind: &str,
    module_path: &str,
    name: &str,
    file: &str,
    span: SourceSpan,
) -> ItemView {
    let id = if module_path.is_empty() {
        name.to_string()
    } else {
        format!("{module_path}::{name}")
    };

    ItemView {
        id: id.clone(),
        kind: kind.to_string(),
        name: name.to_string(),
        container: module_path.to_string(),
        file: file.to_string(),
        span,
        struct_shape: None,
        struct_fields: Vec::new(),
        enum_variants: Vec::new(),
        methods: Vec::new(),
        trait_methods: Vec::new(),
        impl_details: Vec::new(),
        fn_signature: None,
        owner_id: None,
        owner_kind: None,
        source: None,
        trait_id: None,
    }
}

fn finalize_items(nodes: BTreeMap<String, NodeBuilder>) -> BTreeMap<String, ItemView> {
    let mut out = BTreeMap::new();
    for (id, mut node) in nodes {
        let mut item = node.item;
        for trait_method in item.trait_methods {
            node.trait_methods
                .insert(trait_method.name.clone(), trait_method);
        }
        let mut methods = node.method_index.into_values().collect::<Vec<MethodView>>();
        methods.sort_by(|a, b| {
            (
                a.name.as_str(),
                a.source.as_str(),
                a.trait_id.as_deref().unwrap_or(""),
                a.method_id.as_deref().unwrap_or(""),
            )
                .cmp(&(
                    b.name.as_str(),
                    b.source.as_str(),
                    b.trait_id.as_deref().unwrap_or(""),
                    b.method_id.as_deref().unwrap_or(""),
                ))
        });
        item.methods = methods;
        item.trait_methods = node.trait_methods.into_values().collect();
        item.trait_methods
            .sort_by(|a, b| a.name.as_str().cmp(b.name.as_str()));
        item.impl_details = node
            .impl_details
            .into_iter()
            .map(|(trait_id, trait_path)| ImplDetail {
                trait_id,
                trait_path,
            })
            .collect();
        out.insert(id, item);
    }
    out
}

fn build_graph_index(
    items: &BTreeMap<String, ItemView>,
    edge_set: HashSet<(String, String, String, String)>,
) -> GraphIndex {
    let mut nodes = Vec::new();
    let mut by_kind: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut by_container: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for item in items.values() {
        nodes.push(GraphNode {
            id: item.id.clone(),
            kind: item.kind.clone(),
            label: item.name.clone(),
        });
        by_kind
            .entry(item.kind.clone())
            .or_default()
            .push(item.id.clone());
        by_container
            .entry(item.container.clone())
            .or_default()
            .push(item.id.clone());
    }
    for values in by_kind.values_mut() {
        values.sort();
        values.dedup();
    }
    for values in by_container.values_mut() {
        values.sort();
        values.dedup();
    }

    let mut by_edge_kind: BTreeMap<String, Vec<GraphEdgeRef>> = BTreeMap::new();
    let mut edges = edge_set
        .into_iter()
        .map(|(kind, from, to, source_context)| {
            by_edge_kind
                .entry(kind.clone())
                .or_default()
                .push(GraphEdgeRef {
                    from: from.clone(),
                    to: to.clone(),
                });
            GraphEdge {
                kind,
                from,
                to,
                source_context,
            }
        })
        .collect::<Vec<_>>();
    edges.sort_by(|a, b| {
        (
            a.kind.as_str(),
            a.from.as_str(),
            a.to.as_str(),
            a.source_context.as_str(),
        )
            .cmp(&(
                b.kind.as_str(),
                b.from.as_str(),
                b.to.as_str(),
                b.source_context.as_str(),
            ))
    });
    for refs in by_edge_kind.values_mut() {
        refs.sort_by(|a, b| {
            (a.from.as_str(), a.to.as_str()).cmp(&(b.from.as_str(), b.to.as_str()))
        });
        refs.dedup_by(|a, b| a.from == b.from && a.to == b.to);
    }

    GraphIndex {
        nodes,
        edges,
        by_kind,
        by_container,
        by_edge_kind,
    }
}

fn build_crate_views(
    members: &[crate::workspace::WorkspaceMember],
    modules: &BTreeMap<String, ModuleState>,
    items: &BTreeMap<String, ItemView>,
) -> Vec<CrateView> {
    let mut out = Vec::new();
    for member in members {
        let crate_name = member.name.clone();
        let mut module_paths = modules
            .keys()
            .filter(|path| crate_from_module_path(path) == crate_name)
            .cloned()
            .collect::<Vec<_>>();
        if !module_paths.iter().any(|p| p == &crate_name) {
            module_paths.push(crate_name.clone());
        }
        module_paths.sort();
        module_paths.dedup();

        let root = build_module_tree(&crate_name, &module_paths, modules, items);
        out.push(CrateView {
            name: crate_name,
            modules: vec![root],
        });
    }
    out
}

fn build_module_tree(
    current: &str,
    all_paths: &[String],
    modules: &BTreeMap<String, ModuleState>,
    items: &BTreeMap<String, ItemView>,
) -> ModuleView {
    let state = modules.get(current);
    let mut module_items = Vec::new();
    if let Some(state) = state {
        for node_id in &state.node_ids {
            if let Some(item) = items.get(node_id) {
                module_items.push(item.clone());
            }
        }
        module_items.sort_by(|a, b| a.id.cmp(&b.id));
    }

    let mut children = all_paths
        .iter()
        .filter(|path| parent_module(path).as_deref() == Some(current))
        .cloned()
        .collect::<Vec<_>>();
    children.sort();
    children.dedup();

    let modules = children
        .iter()
        .map(|child| build_module_tree(child, all_paths, modules, items))
        .collect::<Vec<_>>();

    let file = state.and_then(|s| s.file.clone());
    ModuleView {
        path: current.to_string(),
        file,
        items: module_items,
        modules,
    }
}

fn parent_module(path: &str) -> Option<String> {
    let mut segs = path.split("::").collect::<Vec<_>>();
    if segs.len() <= 1 {
        return None;
    }
    segs.pop();
    Some(segs.join("::"))
}

fn crate_from_module_path(module_path: &str) -> String {
    module_path
        .split("::")
        .next()
        .map(ToString::to_string)
        .unwrap_or_default()
}

fn source_root_for_file<'a>(
    member: &'a crate::workspace::WorkspaceMember,
    file_path: &Path,
) -> Result<&'a Path, AppError> {
    member
        .source_roots
        .iter()
        .filter(|root| file_path.starts_with(root))
        .max_by_key(|root| root.components().count())
        .map(PathBuf::as_path)
        .ok_or_else(|| {
            AppError::Extract(format!(
                "failed to locate source root for file {} in member `{}`",
                file_path.display(),
                member.name
            ))
        })
}

fn module_segments_from_file(src_dir: &Path, file_path: &Path) -> Result<Vec<String>, AppError> {
    let rel = file_path.strip_prefix(src_dir).map_err(|_| {
        AppError::Extract(format!(
            "failed to compute module path for file {}",
            file_path.display()
        ))
    })?;

    let mut segments = rel
        .parent()
        .map(|parent| {
            parent
                .components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let Some(file_name) = rel.file_name().map(|s| s.to_string_lossy().to_string()) else {
        return Err(AppError::Extract(format!(
            "invalid rust file path {}",
            file_path.display()
        )));
    };
    if file_name != "lib.rs" && file_name != "main.rs" && file_name != "mod.rs" {
        let stem = PathBuf::from(file_name)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .ok_or_else(|| AppError::Extract("invalid module file stem".to_string()))?;
        segments.push(stem);
    }
    Ok(segments)
}

fn to_module_path(crate_name: &str, module_segments: &[String]) -> String {
    if module_segments.is_empty() {
        return crate_name.to_string();
    }
    format!("{crate_name}::{}", module_segments.join("::"))
}

fn span_to_source(span: Span) -> SourceSpan {
    let start = span.start();
    let end = span.end();
    SourceSpan {
        start_line: start.line,
        start_col: start.column,
        end_line: end.line,
        end_col: end.column,
    }
}

fn build_symbol_index(
    nodes: &BTreeMap<String, NodeBuilder>,
) -> (HashSet<String>, HashMap<String, Vec<String>>) {
    let symbol_ids = nodes.keys().cloned().collect::<HashSet<_>>();
    let mut name_index: HashMap<String, Vec<String>> = HashMap::new();
    for id in nodes.keys() {
        let short = id.rsplit("::").next().unwrap_or(id).to_string();
        name_index.entry(short).or_default().push(id.clone());
    }
    for values in name_index.values_mut() {
        values.sort();
        values.dedup();
    }
    (symbol_ids, name_index)
}

fn resolve_item_types(
    nodes: &mut BTreeMap<String, NodeBuilder>,
    module_imports: &HashMap<String, HashMap<String, String>>,
    symbol_ids: &HashSet<String>,
    name_index: &HashMap<String, Vec<String>>,
    workspace_member_names: &HashSet<String>,
) {
    for node in nodes.values_mut() {
        let imports = module_imports
            .get(&node.item.container)
            .cloned()
            .unwrap_or_default();
        let crate_name = crate_from_module_path(&node.item.container);

        for field in &mut node.item.struct_fields {
            if let Some(raw) = &field.target_hint {
                field.type_id = resolve_symbol(
                    raw,
                    &node.item.container,
                    &crate_name,
                    &imports,
                    symbol_ids,
                    name_index,
                    workspace_member_names,
                );
            }
        }

        if let Some(sig) = node.item.fn_signature.as_mut() {
            resolve_fn_signature_types(
                sig,
                &node.item.container,
                &crate_name,
                &imports,
                symbol_ids,
                name_index,
                workspace_member_names,
            );
        }

        for trait_method in &mut node.item.trait_methods {
            resolve_fn_signature_types(
                &mut trait_method.fn_signature,
                &node.item.container,
                &crate_name,
                &imports,
                symbol_ids,
                name_index,
                workspace_member_names,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_fn_signature_types(
    sig: &mut FnSignatureView,
    container: &str,
    crate_name: &str,
    imports: &HashMap<String, String>,
    symbol_ids: &HashSet<String>,
    name_index: &HashMap<String, Vec<String>>,
    workspace_member_names: &HashSet<String>,
) {
    for param in &mut sig.params {
        if let Some(raw) = &param.target_hint {
            param.type_id = resolve_symbol(
                raw,
                container,
                crate_name,
                imports,
                symbol_ids,
                name_index,
                workspace_member_names,
            );
        }
    }

    if let Some(ret) = sig.return_type.as_mut()
        && let Some(raw) = &ret.target_hint
    {
        ret.type_id = resolve_symbol(
            raw,
            container,
            crate_name,
            imports,
            symbol_ids,
            name_index,
            workspace_member_names,
        );
    }
}

fn add_contain_edges(
    nodes: &BTreeMap<String, NodeBuilder>,
    edges: &mut HashSet<(String, String, String, String)>,
) {
    for node in nodes.values() {
        let from = &node.item.id;

        for field in &node.item.struct_fields {
            if let Some(type_id) = &field.type_id
                && type_id != from
            {
                edges.insert((
                    "contain".to_string(),
                    from.clone(),
                    type_id.clone(),
                    "struct_field".to_string(),
                ));
            }
        }

        let Some(sig) = &node.item.fn_signature else {
            continue;
        };

        for param in &sig.params {
            if let Some(type_id) = &param.type_id
                && type_id != from
            {
                edges.insert((
                    "contain".to_string(),
                    from.clone(),
                    type_id.clone(),
                    "fn_param".to_string(),
                ));
            }
        }

        if let Some(ret) = &sig.return_type
            && let Some(type_id) = &ret.type_id
            && type_id != from
        {
            edges.insert((
                "contain".to_string(),
                from.clone(),
                type_id.clone(),
                "fn_return".to_string(),
            ));
        }

        for trait_method in &node.item.trait_methods {
            for param in &trait_method.fn_signature.params {
                if let Some(type_id) = &param.type_id
                    && type_id != from
                {
                    edges.insert((
                        "contain".to_string(),
                        from.clone(),
                        type_id.clone(),
                        "trait_param".to_string(),
                    ));
                }
            }
            if let Some(ret) = &trait_method.fn_signature.return_type
                && let Some(type_id) = &ret.type_id
                && type_id != from
            {
                edges.insert((
                    "contain".to_string(),
                    from.clone(),
                    type_id.clone(),
                    "trait_return".to_string(),
                ));
            }
        }
    }
}

fn method_item_id(
    owner_id: &str,
    method_name: &str,
    source: &str,
    trait_id: Option<&str>,
) -> String {
    match (source, trait_id) {
        ("trait", Some(trait_id)) => {
            format!("{owner_id}::method::trait::{trait_id}::{method_name}")
        }
        _ => format!("{owner_id}::method::inherent::{method_name}"),
    }
}

fn method_index_key(name: &str, source: &str, trait_id: Option<&str>) -> String {
    match (source, trait_id) {
        ("trait", Some(trait_id)) => format!("trait::{trait_id}::{name}"),
        _ => format!("inherent::{name}"),
    }
}

fn build_method_lookup(nodes: &BTreeMap<String, NodeBuilder>) -> MethodLookup {
    let mut by_owner_inherent = HashMap::new();
    let mut by_owner_trait = HashMap::new();
    let mut by_owner_name: HashMap<(String, String), Vec<String>> = HashMap::new();
    let mut callable_ids = HashSet::new();

    for (id, node) in nodes {
        if node.item.kind == "fn" || node.item.kind == "method" {
            callable_ids.insert(id.clone());
        }
        if node.item.kind != "method" {
            continue;
        }
        let Some(owner_id) = node.item.owner_id.clone() else {
            continue;
        };
        let method_name = node.item.name.clone();
        let source = node.item.source.as_deref().unwrap_or("inherent");
        if source == "trait" {
            if let Some(trait_id) = node.item.trait_id.clone() {
                by_owner_trait.insert(
                    (owner_id.clone(), trait_id, method_name.clone()),
                    id.clone(),
                );
            }
        } else {
            by_owner_inherent.insert((owner_id.clone(), method_name.clone()), id.clone());
        }
        by_owner_name
            .entry((owner_id, method_name))
            .or_default()
            .push(id.clone());
    }

    for values in by_owner_name.values_mut() {
        values.sort();
        values.dedup();
    }

    MethodLookup {
        by_owner_inherent,
        by_owner_trait,
        by_owner_name,
        callable_ids,
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_call_edges(
    nodes: &BTreeMap<String, NodeBuilder>,
    module_imports: &HashMap<String, HashMap<String, String>>,
    symbol_ids: &HashSet<String>,
    name_index: &HashMap<String, Vec<String>>,
    workspace_member_names: &HashSet<String>,
    method_lookup: &MethodLookup,
    edges: &mut HashSet<(String, String, String, String)>,
    warnings: &mut Vec<WarningItem>,
) {
    for (from_id, node) in nodes {
        if node.call_specs.is_empty() {
            continue;
        }
        if node.item.kind != "fn" && node.item.kind != "method" {
            continue;
        }

        let imports = module_imports
            .get(&node.item.container)
            .cloned()
            .unwrap_or_default();
        let crate_name = crate_from_module_path(&node.item.container);
        let param_types = build_param_type_index(node.item.fn_signature.as_ref());

        for call in &node.call_specs {
            let should_warn = should_warn_unresolved_call_target(
                &call.target,
                &imports,
                name_index,
                workspace_member_names,
            );
            let resolved = resolve_single_call_target(
                call,
                node,
                &imports,
                &crate_name,
                symbol_ids,
                name_index,
                workspace_member_names,
                method_lookup,
                &param_types,
            );
            if let Some((target_id, source_context)) = resolved {
                if method_lookup.callable_ids.contains(&target_id) {
                    edges.insert((
                        "call".to_string(),
                        from_id.clone(),
                        target_id,
                        source_context,
                    ));
                }
            } else if should_warn {
                warnings.push(warning(
                    "unresolved_call",
                    format!(
                        "cannot resolve call target `{}`",
                        describe_call_target(&call.target)
                    ),
                    Some(call.location.clone()),
                ));
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_single_call_target(
    call: &CallSpec,
    caller: &NodeBuilder,
    imports: &HashMap<String, String>,
    crate_name: &str,
    symbol_ids: &HashSet<String>,
    name_index: &HashMap<String, Vec<String>>,
    workspace_member_names: &HashSet<String>,
    method_lookup: &MethodLookup,
    param_types: &HashMap<String, String>,
) -> Option<(String, String)> {
    match &call.target {
        CallTarget::Path(raw) => resolve_path_call_target(
            raw,
            caller,
            imports,
            crate_name,
            symbol_ids,
            name_index,
            workspace_member_names,
            method_lookup,
        ),
        CallTarget::QualifiedPath {
            self_ty_raw,
            trait_raw,
            method,
        } => {
            let owner_id = resolve_symbol(
                self_ty_raw,
                &caller.item.container,
                crate_name,
                imports,
                symbol_ids,
                name_index,
                workspace_member_names,
            )?;
            let trait_id = trait_raw.as_ref().and_then(|raw| {
                resolve_symbol(
                    raw,
                    &caller.item.container,
                    crate_name,
                    imports,
                    symbol_ids,
                    name_index,
                    workspace_member_names,
                )
            });
            let target =
                resolve_method_target(&owner_id, method, trait_id.as_deref(), method_lookup)?;
            Some((target, "ufcs_call".to_string()))
        }
        CallTarget::Method { method, receiver } => {
            let owner_id = resolve_receiver_owner_id(
                receiver,
                caller,
                imports,
                crate_name,
                symbol_ids,
                name_index,
                workspace_member_names,
                param_types,
            )?;
            let preferred_trait =
                if matches!(receiver, ReceiverHint::SelfValue | ReceiverHint::SelfType) {
                    caller.item.trait_id.as_deref()
                } else {
                    None
                };
            let target = resolve_method_target(&owner_id, method, preferred_trait, method_lookup)?;
            Some((target, "method_call".to_string()))
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_path_call_target(
    raw: &str,
    caller: &NodeBuilder,
    imports: &HashMap<String, String>,
    crate_name: &str,
    symbol_ids: &HashSet<String>,
    name_index: &HashMap<String, Vec<String>>,
    workspace_member_names: &HashSet<String>,
    method_lookup: &MethodLookup,
) -> Option<(String, String)> {
    if let Some(target_id) = resolve_symbol(
        raw,
        &caller.item.container,
        crate_name,
        imports,
        symbol_ids,
        name_index,
        workspace_member_names,
    ) && method_lookup.callable_ids.contains(&target_id)
    {
        return Some((target_id, "fn_call".to_string()));
    }

    let (owner_raw, method_name) = split_owner_and_method(raw)?;
    let owner_id = resolve_symbol(
        &owner_raw,
        &caller.item.container,
        crate_name,
        imports,
        symbol_ids,
        name_index,
        workspace_member_names,
    )?;
    let preferred_trait = if owner_raw == "Self" {
        caller.item.trait_id.as_deref()
    } else {
        None
    };
    let target = resolve_method_target(&owner_id, &method_name, preferred_trait, method_lookup)?;
    Some((target, "associated_call".to_string()))
}

fn resolve_method_target(
    owner_id: &str,
    method_name: &str,
    preferred_trait: Option<&str>,
    method_lookup: &MethodLookup,
) -> Option<String> {
    if let Some(trait_id) = preferred_trait
        && let Some(id) = method_lookup.by_owner_trait.get(&(
            owner_id.to_string(),
            trait_id.to_string(),
            method_name.to_string(),
        ))
    {
        return Some(id.clone());
    }

    if let Some(id) = method_lookup
        .by_owner_inherent
        .get(&(owner_id.to_string(), method_name.to_string()))
    {
        return Some(id.clone());
    }

    let candidates = method_lookup
        .by_owner_name
        .get(&(owner_id.to_string(), method_name.to_string()))?;
    if candidates.len() == 1 {
        return Some(candidates[0].clone());
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn resolve_receiver_owner_id(
    receiver: &ReceiverHint,
    caller: &NodeBuilder,
    imports: &HashMap<String, String>,
    crate_name: &str,
    symbol_ids: &HashSet<String>,
    name_index: &HashMap<String, Vec<String>>,
    workspace_member_names: &HashSet<String>,
    param_types: &HashMap<String, String>,
) -> Option<String> {
    match receiver {
        ReceiverHint::SelfValue | ReceiverHint::SelfType => caller.item.owner_id.clone(),
        ReceiverHint::Ident(name) => param_types.get(name).cloned(),
        ReceiverHint::Path(raw) => resolve_symbol(
            raw,
            &caller.item.container,
            crate_name,
            imports,
            symbol_ids,
            name_index,
            workspace_member_names,
        ),
        ReceiverHint::Unknown => None,
    }
}

fn build_param_type_index(sig: Option<&FnSignatureView>) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Some(sig) = sig else {
        return out;
    };
    for (index, param) in sig.params.iter().enumerate() {
        if let (Some(name), Some(type_id)) = (param.name.clone(), param.type_id.clone()) {
            out.insert(name, type_id);
            continue;
        }
        if let Some(type_id) = param.type_id.clone() {
            out.insert(format!("_arg{}", index + 1), type_id);
        }
    }
    out
}

fn split_owner_and_method(raw: &str) -> Option<(String, String)> {
    let mut parts = raw.rsplitn(2, "::");
    let method = parts.next()?.to_string();
    let owner = parts.next()?.to_string();
    if owner.is_empty() || method.is_empty() {
        return None;
    }
    Some((owner, method))
}

fn should_warn_unresolved_path(
    raw_path: &str,
    imports: &HashMap<String, String>,
    name_index: &HashMap<String, Vec<String>>,
    workspace_member_names: &HashSet<String>,
) -> bool {
    if raw_path.is_empty() {
        return false;
    }
    let first = raw_path.split("::").next().unwrap_or(raw_path);
    if matches!(first, "crate" | "self" | "super" | "Self") {
        return true;
    }
    if workspace_member_names.contains(first) {
        return true;
    }
    if imports.contains_key(first) {
        return true;
    }
    if !raw_path.contains("::")
        && let Some(candidates) = name_index.get(raw_path)
    {
        return !candidates.is_empty();
    }
    false
}

fn should_warn_unresolved_call_target(
    target: &CallTarget,
    imports: &HashMap<String, String>,
    name_index: &HashMap<String, Vec<String>>,
    workspace_member_names: &HashSet<String>,
) -> bool {
    match target {
        CallTarget::Path(raw) => {
            if looks_like_constructor_path(raw) {
                return false;
            }
            should_warn_unresolved_path(raw, imports, name_index, workspace_member_names)
        }
        CallTarget::QualifiedPath {
            self_ty_raw,
            trait_raw,
            ..
        } => {
            should_warn_unresolved_path(self_ty_raw, imports, name_index, workspace_member_names)
                || trait_raw.as_ref().is_some_and(|raw| {
                    should_warn_unresolved_path(raw, imports, name_index, workspace_member_names)
                })
        }
        CallTarget::Method { receiver, .. } => match receiver {
            ReceiverHint::SelfValue | ReceiverHint::SelfType => true,
            ReceiverHint::Path(raw) => {
                should_warn_unresolved_path(raw, imports, name_index, workspace_member_names)
            }
            // Parameter/object methods often target external trait methods (for example `to_string`).
            // Keep call edges when resolvable, but skip unresolved warnings to avoid external noise.
            ReceiverHint::Ident(_name) => false,
            ReceiverHint::Unknown => false,
        },
    }
}

fn looks_like_constructor_path(raw: &str) -> bool {
    let Some((_owner, tail)) = split_owner_and_method(raw) else {
        return false;
    };
    tail.chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
}

fn describe_call_target(target: &CallTarget) -> String {
    match target {
        CallTarget::Path(raw) => raw.clone(),
        CallTarget::QualifiedPath {
            self_ty_raw,
            trait_raw,
            method,
        } => match trait_raw {
            Some(trait_raw) => format!("<{self_ty_raw} as {trait_raw}>::{method}"),
            None => format!("<{self_ty_raw}>::{method}"),
        },
        CallTarget::Method { method, receiver } => {
            let receiver = match receiver {
                ReceiverHint::SelfValue => "self".to_string(),
                ReceiverHint::SelfType => "Self".to_string(),
                ReceiverHint::Path(raw) => raw.clone(),
                ReceiverHint::Ident(name) => name.clone(),
                ReceiverHint::Unknown => "_".to_string(),
            };
            format!("{receiver}.{method}")
        }
    }
}

fn struct_fields_from_syn(fields: &syn::Fields) -> (String, Vec<StructFieldView>) {
    match fields {
        syn::Fields::Named(named) => (
            "named".to_string(),
            named
                .named
                .iter()
                .map(|field| StructFieldView {
                    name: field.ident.as_ref().map(ToString::to_string),
                    index: None,
                    visibility: visibility_to_string(&field.vis),
                    type_expr: type_expr_to_string(&field.ty),
                    type_id: None,
                    target_hint: type_symbol_hint_from_syn(&field.ty),
                })
                .collect(),
        ),
        syn::Fields::Unnamed(unnamed) => (
            "tuple".to_string(),
            unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(index, field)| StructFieldView {
                    name: None,
                    index: Some(index),
                    visibility: visibility_to_string(&field.vis),
                    type_expr: type_expr_to_string(&field.ty),
                    type_id: None,
                    target_hint: type_symbol_hint_from_syn(&field.ty),
                })
                .collect(),
        ),
        syn::Fields::Unit => ("unit".to_string(), Vec::new()),
    }
}

fn visibility_to_string(vis: &syn::Visibility) -> String {
    match vis {
        syn::Visibility::Public(_) => "pub".to_string(),
        syn::Visibility::Restricted(restricted) => {
            format!("pub({})", path_to_string(&restricted.path))
        }
        syn::Visibility::Inherited => "private".to_string(),
    }
}

fn fn_signature_from_sig(sig: &syn::Signature) -> FnSignatureView {
    let params = sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            syn::FnArg::Receiver(_) => None,
            syn::FnArg::Typed(pat) => {
                let name = match pat.pat.as_ref() {
                    syn::Pat::Ident(ident) => Some(ident.ident.to_string()),
                    _ => None,
                };
                Some(FnParamView {
                    name,
                    type_expr: type_expr_to_string(&pat.ty),
                    type_id: None,
                    target_hint: type_symbol_hint_from_syn(&pat.ty),
                })
            }
        })
        .collect::<Vec<_>>();

    let return_type = match &sig.output {
        syn::ReturnType::Default => None,
        syn::ReturnType::Type(_, ty) => Some(FnTypeView {
            type_expr: type_expr_to_string(ty),
            type_id: None,
            target_hint: type_symbol_hint_from_syn(ty),
        }),
    };

    FnSignatureView {
        params,
        return_type,
    }
}

fn trait_method_view_from_syn(method: &syn::TraitItemFn) -> TraitMethodView {
    let receiver = method.sig.inputs.iter().find_map(|arg| match arg {
        syn::FnArg::Receiver(recv) => Some(receiver_to_string(recv)),
        _ => None,
    });
    TraitMethodView {
        name: method.sig.ident.to_string(),
        receiver,
        fn_signature: fn_signature_from_sig(&method.sig),
    }
}

fn receiver_to_string(recv: &syn::Receiver) -> String {
    let mut out = String::new();
    if recv.reference.is_some() {
        out.push('&');
    }
    if recv.mutability.is_some() {
        out.push_str("mut ");
    }
    out.push_str("self");
    out
}

fn collect_call_specs_from_block(block: &syn::Block, file_path: &Path) -> Vec<CallSpec> {
    struct Collector<'a> {
        file_path: &'a Path,
        calls: Vec<CallSpec>,
    }

    impl<'a, 'ast> Visit<'ast> for Collector<'a> {
        fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
            if let Some(target) = call_target_from_callable_expr(node.func.as_ref()) {
                self.calls.push(CallSpec {
                    target,
                    location: format!("{}:{}", self.file_path.display(), node.span().start().line),
                });
            }
            syn::visit::visit_expr_call(self, node);
        }

        fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
            self.calls.push(CallSpec {
                target: CallTarget::Method {
                    method: node.method.to_string(),
                    receiver: receiver_hint_from_expr(node.receiver.as_ref()),
                },
                location: format!("{}:{}", self.file_path.display(), node.span().start().line),
            });
            syn::visit::visit_expr_method_call(self, node);
        }
    }

    let mut collector = Collector {
        file_path,
        calls: Vec::new(),
    };
    collector.visit_block(block);
    collector.calls
}

fn call_target_from_callable_expr(expr: &syn::Expr) -> Option<CallTarget> {
    match expr {
        syn::Expr::Path(path) => {
            if let Some(qself) = &path.qself {
                let self_ty_raw = type_to_string(qself.ty.as_ref())?;
                let segments = path.path.segments.iter().collect::<Vec<_>>();
                let method = segments.last()?.ident.to_string();
                let trait_raw = if segments.len() > 1 {
                    Some(
                        segments[..segments.len() - 1]
                            .iter()
                            .map(|seg| seg.ident.to_string())
                            .collect::<Vec<_>>()
                            .join("::"),
                    )
                } else {
                    None
                };
                Some(CallTarget::QualifiedPath {
                    self_ty_raw,
                    trait_raw,
                    method,
                })
            } else {
                Some(CallTarget::Path(path_to_string(&path.path)))
            }
        }
        syn::Expr::Paren(paren) => call_target_from_callable_expr(&paren.expr),
        syn::Expr::Group(group) => call_target_from_callable_expr(&group.expr),
        syn::Expr::Reference(reference) => call_target_from_callable_expr(&reference.expr),
        _ => None,
    }
}

fn receiver_hint_from_expr(expr: &syn::Expr) -> ReceiverHint {
    match expr {
        syn::Expr::Path(path) => {
            if path.qself.is_some() {
                return ReceiverHint::Path(path_to_string(&path.path));
            }
            let segments = path.path.segments.iter().collect::<Vec<_>>();
            if segments.is_empty() {
                return ReceiverHint::Unknown;
            }
            if segments.len() == 1 {
                let seg = segments[0].ident.to_string();
                if seg == "self" {
                    return ReceiverHint::SelfValue;
                }
                if seg == "Self" {
                    return ReceiverHint::SelfType;
                }
                return ReceiverHint::Ident(seg);
            }
            ReceiverHint::Path(path_to_string(&path.path))
        }
        syn::Expr::Reference(reference) => receiver_hint_from_expr(&reference.expr),
        syn::Expr::Paren(paren) => receiver_hint_from_expr(&paren.expr),
        syn::Expr::Group(group) => receiver_hint_from_expr(&group.expr),
        _ => ReceiverHint::Unknown,
    }
}

fn enum_variants_from_syn(item: &syn::ItemEnum) -> Vec<EnumVariantView> {
    item.variants
        .iter()
        .map(|variant| {
            let (kind, fields) = match &variant.fields {
                syn::Fields::Unit => ("unit".to_string(), Vec::new()),
                syn::Fields::Unnamed(fields) => (
                    "tuple".to_string(),
                    fields
                        .unnamed
                        .iter()
                        .map(|f| VariantFieldView {
                            name: None,
                            ty: type_expr_to_string(&f.ty),
                        })
                        .collect::<Vec<_>>(),
                ),
                syn::Fields::Named(fields) => (
                    "struct".to_string(),
                    fields
                        .named
                        .iter()
                        .map(|f| VariantFieldView {
                            name: f.ident.as_ref().map(ToString::to_string),
                            ty: type_expr_to_string(&f.ty),
                        })
                        .collect::<Vec<_>>(),
                ),
            };
            EnumVariantView {
                name: variant.ident.to_string(),
                kind,
                fields,
            }
        })
        .collect()
}

fn fn_signature_from_syn(item: &syn::ItemFn) -> FnSignatureView {
    fn_signature_from_sig(&item.sig)
}

fn path_to_string(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|seg| seg.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

fn type_to_string(ty: &syn::Type) -> Option<String> {
    type_symbol_hint_from_syn(ty)
}

fn type_expr_to_string(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(tp) => path_to_string(&tp.path),
        syn::Type::Reference(r) => format!("&{}", type_expr_to_string(&r.elem)),
        syn::Type::ImplTrait(it) => {
            let bounds = it
                .bounds
                .iter()
                .filter_map(|b| match b {
                    syn::TypeParamBound::Trait(tb) => Some(path_to_string(&tb.path)),
                    _ => None,
                })
                .collect::<Vec<_>>();
            if bounds.is_empty() {
                "impl _".to_string()
            } else {
                format!("impl {}", bounds.join(" + "))
            }
        }
        syn::Type::Tuple(t) => {
            let elems = t
                .elems
                .iter()
                .map(type_expr_to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({elems})")
        }
        _ => "unsupported".to_string(),
    }
}

fn type_symbol_hint_from_syn(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(tp) => Some(path_to_string(&tp.path)),
        syn::Type::Reference(r) => type_symbol_hint_from_syn(&r.elem),
        syn::Type::Paren(p) => type_symbol_hint_from_syn(&p.elem),
        syn::Type::Group(g) => type_symbol_hint_from_syn(&g.elem),
        syn::Type::ImplTrait(it) => it.bounds.iter().find_map(|bound| match bound {
            syn::TypeParamBound::Trait(tb) => Some(path_to_string(&tb.path)),
            _ => None,
        }),
        _ => None,
    }
}

fn collect_use_specs(tree: &syn::UseTree, prefix: String, out: &mut Vec<(String, Option<String>)>) {
    match tree {
        syn::UseTree::Path(p) => {
            let next = if prefix.is_empty() {
                p.ident.to_string()
            } else {
                format!("{prefix}::{}", p.ident)
            };
            collect_use_specs(&p.tree, next, out);
        }
        syn::UseTree::Name(n) => {
            let raw = if prefix.is_empty() {
                n.ident.to_string()
            } else {
                format!("{prefix}::{}", n.ident)
            };
            out.push((raw, None));
        }
        syn::UseTree::Rename(r) => {
            let raw = if prefix.is_empty() {
                r.ident.to_string()
            } else {
                format!("{prefix}::{}", r.ident)
            };
            out.push((raw, Some(r.rename.to_string())));
        }
        syn::UseTree::Group(g) => {
            for tree in &g.items {
                collect_use_specs(tree, prefix.clone(), out);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

fn resolve_symbol(
    raw_path: &str,
    module_path: &str,
    crate_name: &str,
    imports: &HashMap<String, String>,
    symbol_ids: &HashSet<String>,
    name_index: &HashMap<String, Vec<String>>,
    workspace_member_names: &HashSet<String>,
) -> Option<String> {
    if raw_path.is_empty() {
        return None;
    }

    let segments = raw_path.split("::").collect::<Vec<_>>();
    if segments.is_empty() {
        return None;
    }

    let mut candidates = Vec::<String>::new();
    if let Some(import_target) = imports.get(segments[0]) {
        if segments.len() == 1 {
            candidates.push(import_target.clone());
        } else {
            candidates.push(format!("{import_target}::{}", segments[1..].join("::")));
        }
    }

    if segments[0] == "crate" {
        if segments.len() > 1 {
            candidates.push(format!("{crate_name}::{}", segments[1..].join("::")));
        } else {
            candidates.push(crate_name.to_string());
        }
    } else if segments[0] == "self" {
        if segments.len() > 1 {
            candidates.push(format!("{module_path}::{}", segments[1..].join("::")));
        } else {
            candidates.push(module_path.to_string());
        }
    } else if segments[0] == "super" {
        let mut parent = module_path.to_string();
        let mut idx = 0usize;
        while idx < segments.len() && segments[idx] == "super" {
            if let Some(p) = parent_module(&parent) {
                parent = p;
            }
            idx += 1;
        }
        if idx < segments.len() {
            candidates.push(format!("{parent}::{}", segments[idx..].join("::")));
        } else {
            candidates.push(parent);
        }
    } else {
        if workspace_member_names.contains(segments[0]) {
            candidates.push(raw_path.to_string());
        }
        candidates.push(format!("{module_path}::{raw_path}"));
        candidates.push(format!("{crate_name}::{raw_path}"));
        candidates.push(raw_path.to_string());
    }

    for cand in candidates {
        if symbol_ids.contains(&cand) {
            return Some(cand);
        }
    }

    if !raw_path.contains("::")
        && let Some(candidates) = name_index.get(raw_path)
        && candidates.len() == 1
    {
        return Some(candidates[0].clone());
    }

    None
}

fn last_segment(path: &str) -> &str {
    path.rsplit("::").next().unwrap_or(path)
}

fn warning(code: &str, message: String, location: Option<String>) -> WarningItem {
    WarningItem {
        code: code.to_string(),
        severity: "warn".to_string(),
        message,
        location,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::workspace;

    use super::extract_workspace;

    #[test]
    fn extract_fixture_workspace() {
        let snapshot = workspace::load(Path::new("examples/workspace_demo"))
            .expect("workspace snapshot should load");
        let output = extract_workspace(&snapshot).expect("extract should succeed");

        assert_eq!(output.workspace.members.len(), 2);
        assert!(
            output
                .graph_index
                .nodes
                .iter()
                .any(|n| n.kind == "struct" && n.label == "User")
        );
        assert!(
            output.graph_index.edges.iter().any(|e| e.kind == "inherit"),
            "should include trait inherit edge"
        );
        assert!(
            output.graph_index.edges.iter().any(|e| e.kind == "impl"),
            "should include impl edge"
        );
        assert!(
            output.graph_index.edges.iter().any(|e| e.kind == "call"),
            "should include call edge"
        );
        assert!(
            output.graph_index.nodes.iter().any(|n| n.kind == "method"),
            "should include method node"
        );
        let named_trait = output
            .crates
            .iter()
            .flat_map(|crate_view| crate_view.modules.iter())
            .flat_map(|module_view| module_view.items.iter())
            .find(|item| item.id == "core_types::Named")
            .expect("fixture should include trait Named");
        let name_method = named_trait
            .trait_methods
            .iter()
            .find(|method| method.name == "name")
            .expect("trait method metadata should be present");
        assert_eq!(name_method.receiver.as_deref(), Some("&self"));
        assert!(
            name_method.fn_signature.return_type.is_some(),
            "trait method should include return metadata"
        );
    }
}
