use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}_{nanos}"))
}

#[test]
fn writes_default_shape_and_overwrites() {
    let fixture = PathBuf::from("examples/workspace_demo");
    let temp = unique_temp_dir("rustmap_phase1");
    fs::create_dir_all(&temp).expect("temp directory should be created");

    let output = temp.join("out.json");
    let first = rustmap::run(&fixture, &output).expect("first run should succeed");
    assert!(output.exists(), "output file should exist");
    assert_eq!(first.workspace.members.len(), 2);

    rustmap::run(&fixture, &output).expect("second run should overwrite output");
    let written = fs::read_to_string(&output).expect("output should be readable");
    let json: serde_json::Value =
        serde_json::from_str(&written).expect("written output should be valid json");
    assert!(json.get("workspace").is_some());
    assert!(json.get("dependencies").is_some());
    assert!(json.get("crates").is_some());
    assert!(json.get("graph_index").is_some());
    assert!(json.get("warnings").is_some());
    assert!(
        json.get("graph_index")
            .and_then(|v| v.get("by_edge_kind"))
            .is_some(),
        "graph_index.by_edge_kind should be present"
    );

    let role_node = first
        .crates
        .iter()
        .flat_map(|c| c.modules.iter())
        .flat_map(|m| m.items.iter())
        .find(|item| item.id == "core_types::Role")
        .expect("enum Role should exist");
    assert_eq!(role_node.kind, "enum");
    assert_eq!(role_node.enum_variants.len(), 2);
    assert!(role_node.enum_variants.iter().any(|v| v.name == "Admin"));
    assert!(role_node.enum_variants.iter().any(|v| v.name == "User"));

    let named_trait = first
        .crates
        .iter()
        .flat_map(|c| c.modules.iter())
        .flat_map(|m| m.items.iter())
        .find(|item| item.id == "core_types::Named")
        .expect("trait Named should exist");
    let name_method = named_trait
        .trait_methods
        .iter()
        .find(|method| method.name == "name")
        .expect("trait Named should expose method signature");
    assert_eq!(name_method.receiver.as_deref(), Some("&self"));
    assert!(name_method.fn_signature.params.is_empty());
    assert_eq!(
        name_method
            .fn_signature
            .return_type
            .as_ref()
            .map(|ret| ret.type_expr.as_str()),
        Some("&str")
    );

    let user_node = first
        .crates
        .iter()
        .flat_map(|c| c.modules.iter())
        .flat_map(|m| m.items.iter())
        .find(|item| item.id == "core_types::User")
        .expect("struct User should exist");
    assert_eq!(user_node.struct_shape.as_deref(), Some("named"));
    assert_eq!(user_node.struct_fields.len(), 3);
    assert!(user_node.struct_fields.iter().any(|f| {
        f.name.as_deref() == Some("role") && f.type_id.as_deref() == Some("core_types::Role")
    }));
    assert!(user_node.methods.iter().any(|m| {
        m.name == "id" && m.source == "trait" && m.trait_id.as_deref() == Some("core_types::Entity")
    }));
    assert!(user_node.methods.iter().any(|m| {
        m.name == "name"
            && m.source == "trait"
            && m.trait_id.as_deref() == Some("core_types::Named")
    }));
    assert!(user_node.methods.iter().all(|m| m.method_id.is_some()));

    let user_id_node = first
        .crates
        .iter()
        .flat_map(|c| c.modules.iter())
        .flat_map(|m| m.items.iter())
        .find(|item| item.id == "core_types::UserId")
        .expect("tuple struct UserId should exist");
    assert_eq!(user_id_node.struct_shape.as_deref(), Some("tuple"));
    assert_eq!(user_id_node.struct_fields.len(), 1);
    assert_eq!(user_id_node.struct_fields[0].index, Some(0));
    assert_eq!(user_id_node.struct_fields[0].type_expr, "u64");

    let marker_node = first
        .crates
        .iter()
        .flat_map(|c| c.modules.iter())
        .flat_map(|m| m.items.iter())
        .find(|item| item.id == "core_types::UserMarker")
        .expect("unit struct UserMarker should exist");
    assert_eq!(marker_node.struct_shape.as_deref(), Some("unit"));
    assert!(marker_node.struct_fields.is_empty());

    let free_fn = first
        .crates
        .iter()
        .flat_map(|c| c.modules.iter())
        .flat_map(|m| m.items.iter())
        .find(|item| item.id == "core_types::display_name")
        .expect("free fn display_name should exist");
    let sig = free_fn
        .fn_signature
        .as_ref()
        .expect("free function should include signature");
    assert!(!sig.params.is_empty(), "signature params should be present");
    assert!(
        sig.params
            .iter()
            .any(|p| p.type_id.as_deref() == Some("core_types::Named")),
        "function param should resolve to trait node id"
    );
    assert!(
        first.graph_index.nodes.iter().any(|n| n.kind == "method"),
        "graph should include method nodes"
    );
    assert!(
        first.graph_index.edges.iter().any(|e| {
            e.kind == "call"
                && e.from == "service_app::build_report"
                && e.to == "core_types::display_name"
        }),
        "should include fn -> fn call edge"
    );
    assert!(
        first.graph_index.edges.iter().any(|e| {
            e.kind == "call"
                && e.from == "service_app::build_report"
                && e.to == "core_types::User::method::inherent::report_line"
        }),
        "should include fn -> method call edge"
    );
    assert!(
        first.graph_index.edges.iter().any(|e| {
            e.kind == "call"
                && e.from == "core_types::User::method::inherent::report_line"
                && e.to == "core_types::User::method::inherent::tag"
        }),
        "should include method -> method call edge"
    );
    assert!(
        first
            .graph_index
            .by_edge_kind
            .get("call")
            .is_some_and(|edges| !edges.is_empty()),
        "call edges should be indexed by kind"
    );

    assert!(
        first
            .graph_index
            .edges
            .iter()
            .all(|e| !e.source_context.is_empty()),
        "all edges should include source_context"
    );
    assert!(
        first
            .graph_index
            .edges
            .iter()
            .all(|e| e.source_context != "module_use"),
        "module-level use imports should not be emitted as graph edges"
    );
    assert!(
        first.warnings.iter().all(|w| w.severity == "warn"),
        "warnings should include severity"
    );
    assert!(
        first.graph_index.by_edge_kind.contains_key("impl")
            && first.graph_index.by_edge_kind.contains_key("contain")
            && first.graph_index.by_edge_kind.contains_key("inherit")
            && first.graph_index.by_edge_kind.contains_key("call"),
        "by_edge_kind should group the known edge kinds"
    );
}

#[test]
fn invalid_input_path_fails() {
    let missing = PathBuf::from("examples/this_does_not_exist");
    let out = unique_temp_dir("rustmap_phase1_fail").join("out.json");
    let err = rustmap::run(&missing, &out).expect_err("missing path should fail");
    let message = err.to_string();
    assert!(
        message.contains("cannot find Cargo.toml"),
        "error should explain missing Cargo.toml"
    );
}

#[test]
fn workspace_collects_custom_target_source_roots() {
    let snapshot = rustmap::workspace::load(Path::new("."))
        .expect("workspace metadata should load from repo root");
    let member = snapshot
        .members
        .iter()
        .find(|m| m.name == "rustmap")
        .expect("root crate rustmap should exist");

    assert!(
        member.source_roots.iter().any(|root| root.ends_with("src")),
        "root bin source dir should be included"
    );
    assert!(
        member
            .source_roots
            .iter()
            .any(|root| root.ends_with("artifacts/parser/src")),
        "custom lib source dir should be included"
    );
    assert!(
        member
            .rust_files
            .iter()
            .any(|file| file.ends_with("artifacts/parser/src/extract.rs")),
        "parser source files should be collected"
    );
}
