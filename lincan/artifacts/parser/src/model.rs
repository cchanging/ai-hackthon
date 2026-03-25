use std::collections::BTreeMap;

#[derive(Debug, Clone, serde::Serialize)]
pub struct RustMapOutput {
    pub workspace: WorkspaceInfo,
    pub dependencies: Vec<DependencyInfo>,
    pub crates: Vec<CrateView>,
    pub graph_index: GraphIndex,
    pub warnings: Vec<WarningItem>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkspaceInfo {
    pub root: String,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DependencyInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CrateView {
    pub name: String,
    pub modules: Vec<ModuleView>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ModuleView {
    pub path: String,
    pub file: Option<String>,
    pub items: Vec<ItemView>,
    pub modules: Vec<ModuleView>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ItemView {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub container: String,
    pub file: String,
    pub span: SourceSpan,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub struct_shape: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub struct_fields: Vec<StructFieldView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub enum_variants: Vec<EnumVariantView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<MethodView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub trait_methods: Vec<TraitMethodView>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub impl_details: Vec<ImplDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fn_signature: Option<FnSignatureView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trait_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ImplDetail {
    pub trait_id: String,
    pub trait_path: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SourceSpan {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EnumVariantView {
    pub name: String,
    pub kind: String,
    pub fields: Vec<VariantFieldView>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VariantFieldView {
    pub name: Option<String>,
    pub ty: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StructFieldView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<usize>,
    pub visibility: String,
    pub type_expr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_id: Option<String>,
    #[serde(skip_serializing)]
    pub target_hint: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MethodView {
    pub name: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trait_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TraitMethodView {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receiver: Option<String>,
    pub fn_signature: FnSignatureView,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FnSignatureView {
    pub params: Vec<FnParamView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_type: Option<FnTypeView>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FnParamView {
    pub name: Option<String>,
    pub type_expr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_id: Option<String>,
    #[serde(skip_serializing)]
    pub target_hint: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FnTypeView {
    pub type_expr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_id: Option<String>,
    #[serde(skip_serializing)]
    pub target_hint: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphIndex {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub by_kind: BTreeMap<String, Vec<String>>,
    pub by_container: BTreeMap<String, Vec<String>>,
    pub by_edge_kind: BTreeMap<String, Vec<GraphEdgeRef>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphNode {
    pub id: String,
    pub kind: String,
    pub label: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphEdge {
    pub kind: String,
    pub from: String,
    pub to: String,
    pub source_context: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GraphEdgeRef {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WarningItem {
    pub code: String,
    pub severity: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}
