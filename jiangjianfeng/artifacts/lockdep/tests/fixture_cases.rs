// SPDX-License-Identifier: MPL-2.0

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;

fn fixture_manifest() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("ostd-lockdep-cases")
        .join("Cargo.toml")
}

fn temp_json_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("lockdep-fixture-{nanos}.json"))
}

fn run_lockdep_fixture() -> Value {
    let output_json = temp_json_path();
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-lockdep"))
        .arg("--manifest-path")
        .arg(fixture_manifest())
        .arg("--target")
        .arg("x86_64-unknown-none")
        .arg("--emit-json")
        .arg(&output_json)
        .arg("--")
        .arg("--quiet")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let json = fs::read_to_string(&output_json).unwrap();
    let _ = fs::remove_file(output_json);
    serde_json::from_str(&json).unwrap()
}

fn fixture_summary() -> &'static Value {
    static SUMMARY: OnceLock<Value> = OnceLock::new();
    SUMMARY.get_or_init(run_lockdep_fixture)
}

fn functions(summary: &Value) -> &[Value] {
    summary["crates"][0]["functions"].as_array().unwrap()
}

fn find_function<'a>(summary: &'a Value, suffix: &str) -> &'a Value {
    functions(summary)
        .iter()
        .find(|function| function["def_path"].as_str().unwrap().ends_with(suffix))
        .unwrap_or_else(|| panic!("missing function with suffix {suffix}"))
}

fn first_lock_class<'a>(summary: &'a Value, suffix: &str) -> &'a Value {
    &find_function(summary, suffix)["lock_events"][0]["lock"]["class"]
}

fn root_variant<'a>(class: &'a Value, variant: &str) -> &'a Value {
    &class["root"][variant]
}

fn projections(class: &Value) -> &[Value] {
    class["projections"].as_array().unwrap()
}

fn find_projection<'a>(class: &'a Value, variant: &str) -> &'a Value {
    projections(class)
        .iter()
        .find(|projection| projection.get(variant).is_some())
        .unwrap_or_else(|| panic!("missing projection variant {variant}"))
}

#[test]
fn reports_expected_global_counts() {
    let summary = fixture_summary();

    assert_eq!(summary["global_report"]["cycle_count"].as_u64().unwrap(), 4);
    assert_eq!(
        summary["global_report"]["atomic_mode_violation_count"]
            .as_u64()
            .unwrap(),
        3
    );
    assert_eq!(
        summary["global_report"]["single_lock_irq_violation_count"]
            .as_u64()
            .unwrap(),
        3
    );
    assert_eq!(
        summary["global_report"]["irq_dependency_violation_count"]
            .as_u64()
            .unwrap(),
        1
    );
    assert_eq!(
        summary["global_report"]["irq_conflict_count"]
            .as_u64()
            .unwrap(),
        3
    );
    assert_eq!(
        summary["global_report"]["aa_deadlock_count"]
            .as_u64()
            .unwrap(),
        4
    );
}

#[test]
fn propagates_irq_contexts_and_usage_bits() {
    let summary = fixture_summary();

    let l1_handler = find_function(summary, "bottom_half_l1");
    assert!(
        l1_handler["contexts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("BottomHalfL1"))
    );
    assert!(
        l1_handler["lock_events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["context"].as_str() == Some("BottomHalfL1IrqDisabled"))
    );

    let l2_worker = find_function(summary, "bottom_half_l2_worker");
    assert!(
        l2_worker["contexts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("BottomHalfL2"))
    );
    assert!(
        l2_worker["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| state["bits"]["used_in_hardirq"].as_bool() == Some(true))
    );

    let l1_worker = find_function(summary, "bottom_half_l1_worker");
    assert!(
        l1_worker["contexts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("BottomHalfL1"))
    );
    assert!(
        l1_worker["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| {
                state["bits"]["used_in_softirq"].as_bool() == Some(true)
                    && state["bits"]["used_with_hardirq_enabled"].as_bool() == Some(true)
            })
    );

    let l1_guard_move = find_function(summary, "bottom_half_l1_guard_move");
    assert!(
        l1_guard_move["contexts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("BottomHalfL1"))
    );
    assert!(
        l1_guard_move["lock_events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["context"].as_str() == Some("BottomHalfL1IrqDisabled"))
    );
    assert!(
        l1_guard_move["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| {
                state["bits"]["used_in_softirq"].as_bool() == Some(true)
                    && state["bits"]["used_with_hardirq_disabled"].as_bool() == Some(true)
                    && state["bits"]["used_with_softirq_disabled"].as_bool() == Some(true)
            })
    );

    let top_half_worker = find_function(summary, "irq_conflict_worker");
    assert!(
        top_half_worker["contexts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("HardIrqTopHalf"))
    );
    assert!(
        top_half_worker["lock_events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["context"].as_str() == Some("HardIrqTopHalf"))
    );
    assert!(
        top_half_worker["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| state["bits"]["used_in_hardirq"].as_bool() == Some(true))
    );

    let configured_worker = find_function(summary, "configured_irq_worker");
    assert!(
        configured_worker["contexts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("HardIrqTopHalf"))
    );
    assert!(
        configured_worker["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| state["bits"]["used_in_hardirq"].as_bool() == Some(true))
    );

    let configured_softirq_worker = find_function(summary, "configured_softirq_worker");
    assert!(
        configured_softirq_worker["contexts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("BottomHalfL1"))
    );
    assert!(
        configured_softirq_worker["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| {
                state["bits"]["used_in_softirq"].as_bool() == Some(true)
                    && state["bits"]["used_with_softirq_enabled"].as_bool() == Some(true)
            })
    );

    let disable_irq_fn = find_function(summary, "task_disable_irq_lock");
    assert!(
        disable_irq_fn["lock_events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["context"].as_str() == Some("TaskIrqDisabled"))
    );
    assert!(
        disable_irq_fn["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| state["bits"]["used_with_hardirq_disabled"].as_bool() == Some(true))
    );

    let restore_irq_fn = find_function(summary, "task_disable_irq_then_plain_lock");
    let restore_events = restore_irq_fn["lock_events"].as_array().unwrap();
    assert_eq!(restore_events.len(), 4);
    assert_eq!(restore_events[0]["kind"].as_str(), Some("acquire"));
    assert_eq!(
        restore_events[0]["context"].as_str(),
        Some("TaskIrqDisabled")
    );
    assert_eq!(restore_events[1]["kind"].as_str(), Some("release"));
    assert_eq!(
        restore_events[1]["context"].as_str(),
        Some("TaskIrqDisabled")
    );
    assert_eq!(restore_events[2]["kind"].as_str(), Some("acquire"));
    assert_eq!(restore_events[2]["context"].as_str(), Some("Task"));
    assert_eq!(restore_events[3]["kind"].as_str(), Some("release"));
    assert_eq!(restore_events[3]["context"].as_str(), Some("Task"));

    let irq_disabled_shared = find_function(summary, "shared_irq_disabled_worker");
    assert!(
        irq_disabled_shared["contexts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("HardIrqTopHalf"))
    );
    assert!(
        irq_disabled_shared["contexts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("Task"))
    );
    assert!(
        irq_disabled_shared["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| {
                state["bits"]["used_in_hardirq"].as_bool() == Some(true)
                    && state["bits"]["used_with_hardirq_disabled"].as_bool() == Some(true)
                    && state["bits"]["used_with_hardirq_enabled"].as_bool() == Some(false)
            })
    );

    let task_branch_irq_merge = find_function(summary, "task_branch_irq_merge");
    assert!(
        task_branch_irq_merge["lock_events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| {
                event["kind"].as_str() == Some("acquire")
                    && event["context"].as_str() == Some("Task")
                    && event["lock"]["class"]["root"]["Global"]["def_path"]
                        .as_str()
                        .is_some_and(|path| path.ends_with("JOIN_MERGE_IRQ_LOCK"))
            })
    );
    assert!(
        task_branch_irq_merge["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| {
                state["lock"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("JOIN_MERGE_IRQ_LOCK"))
                    && state["bits"]["used_with_hardirq_enabled"].as_bool() == Some(true)
            })
    );

    let propagated_plain = find_function(summary, "call_propagated_helper_plain");
    assert!(
        propagated_plain["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| {
                state["lock"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("PROPAGATED_HELPER_LOCK"))
                    && state["bits"]["used_with_hardirq_enabled"].as_bool() == Some(true)
            })
    );

    let indirect_propagated_plain = find_function(summary, "call_indirect_propagated_helper_plain");
    assert!(
        indirect_propagated_plain["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| {
                state["lock"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("PROPAGATED_HELPER_LOCK"))
                    && state["bits"]["used_with_hardirq_enabled"].as_bool() == Some(true)
            })
    );

    let propagated_helper = find_function(summary, "propagated_helper_lock");
    assert!(
        propagated_helper["contexts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("Task"))
    );
    assert!(
        propagated_helper["contexts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("TaskIrqDisabled"))
    );
    assert!(
        propagated_helper["lock_events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["context"].as_str() == Some("TaskIrqDisabled"))
    );

    let propagated_irq_disabled = find_function(summary, "call_propagated_helper_irq_disabled");
    assert!(
        propagated_irq_disabled["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| {
                state["lock"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("PROPAGATED_HELPER_LOCK"))
                    && state["bits"]["used_with_hardirq_disabled"].as_bool() == Some(true)
                    && state["first_hardirq_disabled_use"]["context"].as_str()
                        == Some("TaskIrqDisabled")
            })
    );

    let indirect_propagated_irq_disabled =
        find_function(summary, "call_indirect_propagated_helper_irq_disabled");
    assert!(
        indirect_propagated_irq_disabled["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| {
                state["lock"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("PROPAGATED_HELPER_LOCK"))
                    && state["bits"]["used_with_hardirq_disabled"].as_bool() == Some(true)
                    && state["first_hardirq_disabled_use"]["context"].as_str()
                        == Some("TaskIrqDisabled")
            })
    );

    let wrapped_callback_target = find_function(summary, "wrapped_callback_target");
    assert!(
        wrapped_callback_target["contexts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("Task"))
    );
    assert!(
        wrapped_callback_target["contexts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("TaskIrqDisabled"))
    );
    assert!(
        wrapped_callback_target["lock_events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["context"].as_str() == Some("TaskIrqDisabled"))
    );

    let wrapped_callback_plain = find_function(summary, "call_wrapped_callback_plain");
    assert!(
        wrapped_callback_plain["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| {
                state["lock"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("WRAPPED_CALLBACK_LOCK"))
                    && state["bits"]["used_with_hardirq_enabled"].as_bool() == Some(true)
            })
    );

    let wrapped_callback_alias_plain = find_function(summary, "call_wrapped_callback_alias_plain");
    assert!(
        wrapped_callback_alias_plain["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| {
                state["lock"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("WRAPPED_CALLBACK_LOCK"))
                    && state["bits"]["used_with_hardirq_enabled"].as_bool() == Some(true)
            })
    );

    let wrapped_callback_irq_disabled =
        find_function(summary, "call_wrapped_callback_irq_disabled");
    assert!(
        wrapped_callback_irq_disabled["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| {
                state["lock"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("WRAPPED_CALLBACK_LOCK"))
                    && state["bits"]["used_with_hardirq_disabled"].as_bool() == Some(true)
                    && state["first_hardirq_disabled_use"]["context"].as_str()
                        == Some("TaskIrqDisabled")
            })
    );

    let wrapped_callback_alias_irq_disabled =
        find_function(summary, "call_wrapped_callback_alias_irq_disabled");
    assert!(
        wrapped_callback_alias_irq_disabled["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| {
                state["lock"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("WRAPPED_CALLBACK_LOCK"))
                    && state["bits"]["used_with_hardirq_disabled"].as_bool() == Some(true)
                    && state["first_hardirq_disabled_use"]["context"].as_str()
                        == Some("TaskIrqDisabled")
            })
    );

    let outer_wrapped_callback_plain = find_function(summary, "call_outer_wrapped_callback_plain");
    assert!(
        outer_wrapped_callback_plain["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| {
                state["lock"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("WRAPPED_CALLBACK_LOCK"))
                    && state["bits"]["used_with_hardirq_enabled"].as_bool() == Some(true)
            })
    );

    let outer_wrapped_callback_irq_disabled =
        find_function(summary, "call_outer_wrapped_callback_irq_disabled");
    assert!(
        outer_wrapped_callback_irq_disabled["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| {
                state["lock"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("WRAPPED_CALLBACK_LOCK"))
                    && state["bits"]["used_with_hardirq_disabled"].as_bool() == Some(true)
                    && state["first_hardirq_disabled_use"]["context"].as_str()
                        == Some("TaskIrqDisabled")
            })
    );

    let wrapped_callback_with_held_lock =
        find_function(summary, "call_wrapped_callback_with_held_lock");
    assert!(
        wrapped_callback_with_held_lock["lock_edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| {
                edge["context"].as_str() == Some("Task")
                    && edge["from"]["class"]["root"]["Global"]["def_path"]
                        .as_str()
                        .is_some_and(|path| path.ends_with("LOCK_B"))
                    && edge["to"]["class"]["root"]["Global"]["def_path"]
                        .as_str()
                        .is_some_and(|path| path.ends_with("WRAPPED_CALLBACK_LOCK"))
            })
    );

    let callback_wrapper_holding = find_function(summary, "invoke_callback_while_holding");
    assert!(
        callback_wrapper_holding["contexts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("Task"))
    );
    assert!(
        callback_wrapper_holding["contexts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("TaskIrqDisabled"))
    );
    assert!(
        callback_wrapper_holding["lock_edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| {
                edge["context"].as_str() == Some("Task")
                    && edge["from"]["class"]["root"]["Global"]["def_path"]
                        .as_str()
                        .is_some_and(|path| path.ends_with("HELPER_LOCK_A"))
                    && edge["to"]["class"]["root"]["Global"]["def_path"]
                        .as_str()
                        .is_some_and(|path| path.ends_with("WRAPPED_CALLBACK_LOCK"))
            })
    );
    assert!(
        callback_wrapper_holding["lock_edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| {
                edge["context"].as_str() == Some("TaskIrqDisabled")
                    && edge["from"]["class"]["root"]["Global"]["def_path"]
                        .as_str()
                        .is_some_and(|path| path.ends_with("HELPER_LOCK_A"))
                    && edge["to"]["class"]["root"]["Global"]["def_path"]
                        .as_str()
                        .is_some_and(|path| path.ends_with("WRAPPED_CALLBACK_LOCK"))
            })
    );

    let wrapped_callback_with_held_lock_irq_disabled =
        find_function(summary, "call_wrapped_callback_with_held_lock_irq_disabled");
    assert!(
        wrapped_callback_with_held_lock_irq_disabled["lock_edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|edge| {
                edge["context"].as_str() == Some("TaskIrqDisabled")
                    && edge["from"]["guard_behavior"].as_str() == Some("LocalIrqDisabled")
                    && edge["to"]["class"]["root"]["Global"]["def_path"]
                        .as_str()
                        .is_some_and(|path| path.ends_with("WRAPPED_CALLBACK_LOCK"))
            })
    );

    let wrapped_callback_with_lock = find_function(summary, "wrapped_callback_with_lock");
    assert!(
        wrapped_callback_with_lock["contexts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("Task"))
    );
    assert!(
        wrapped_callback_with_lock["contexts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value.as_str() == Some("TaskIrqDisabled"))
    );

    let wrapped_callback_with_lock_plain =
        find_function(summary, "call_wrapped_callback_with_lock_plain");
    assert!(
        wrapped_callback_with_lock_plain["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| {
                state["lock"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("WRAPPED_CALLBACK_ARG_LOCK"))
                    && state["bits"]["used_with_hardirq_enabled"].as_bool() == Some(true)
            })
    );

    let wrapped_callback_with_lock_alias_plain =
        find_function(summary, "call_wrapped_callback_with_lock_alias_plain");
    assert!(
        wrapped_callback_with_lock_alias_plain["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| {
                state["lock"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("WRAPPED_CALLBACK_ARG_LOCK"))
                    && state["bits"]["used_with_hardirq_enabled"].as_bool() == Some(true)
            })
    );

    let wrapped_callback_with_lock_irq_disabled =
        find_function(summary, "call_wrapped_callback_with_lock_irq_disabled");
    assert!(
        wrapped_callback_with_lock_irq_disabled["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| {
                state["lock"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("WRAPPED_CALLBACK_ARG_LOCK"))
                    && state["bits"]["used_with_hardirq_disabled"].as_bool() == Some(true)
                    && state["first_hardirq_disabled_use"]["context"].as_str()
                        == Some("TaskIrqDisabled")
            })
    );

    let wrapped_callback_with_lock_alias_irq_disabled = find_function(
        summary,
        "call_wrapped_callback_with_lock_alias_irq_disabled",
    );
    assert!(
        wrapped_callback_with_lock_alias_irq_disabled["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| {
                state["lock"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("WRAPPED_CALLBACK_ARG_LOCK"))
                    && state["bits"]["used_with_hardirq_disabled"].as_bool() == Some(true)
                    && state["first_hardirq_disabled_use"]["context"].as_str()
                        == Some("TaskIrqDisabled")
            })
    );

    let outer_wrapped_callback_with_lock_plain =
        find_function(summary, "call_outer_wrapped_callback_with_lock_plain");
    assert!(
        outer_wrapped_callback_with_lock_plain["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| {
                state["lock"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("WRAPPED_CALLBACK_ARG_LOCK"))
                    && state["bits"]["used_with_hardirq_enabled"].as_bool() == Some(true)
            })
    );

    let outer_wrapped_callback_with_lock_irq_disabled = find_function(
        summary,
        "call_outer_wrapped_callback_with_lock_irq_disabled",
    );
    assert!(
        outer_wrapped_callback_with_lock_irq_disabled["lock_usage_states"]
            .as_array()
            .unwrap()
            .iter()
            .any(|state| {
                state["lock"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("WRAPPED_CALLBACK_ARG_LOCK"))
                    && state["bits"]["used_with_hardirq_disabled"].as_bool() == Some(true)
                    && state["first_hardirq_disabled_use"]["context"].as_str()
                        == Some("TaskIrqDisabled")
            })
    );
}

#[test]
fn tracks_lock_modes_and_returned_guards() {
    let summary = fixture_summary();

    let rw_read = find_function(summary, "rw_read_path");
    assert!(
        rw_read["lock_events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| {
                event["lock"]["primitive"].as_str() == Some("RwLock")
                    && event["lock"]["guard_behavior"].as_str() == Some("PreemptDisabled")
            })
    );

    let rw_write = find_function(summary, "rw_write_path");
    assert!(
        rw_write["lock_events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| {
                event["lock"]["primitive"].as_str() == Some("RwLock")
                    && event["lock"]["guard_behavior"].as_str() == Some("WriteIrqDisabled")
            })
    );

    let returned_guard_path = find_function(summary, "returned_guard_path");
    let returned_guard_edge = returned_guard_path["lock_edges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|edge| {
            edge["from"]["class"]["root"]["Global"]["def_path"]
                .as_str()
                .is_some_and(|path| path.ends_with("RETURN_LOCK_A"))
                && edge["to"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("RETURN_LOCK_B"))
        });
    assert!(returned_guard_edge.is_some(), "missing returned-guard edge");

    let indirect_returned_guard_path = find_function(summary, "indirect_returned_guard_path");
    let indirect_returned_guard_edge = indirect_returned_guard_path["lock_edges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|edge| {
            edge["from"]["class"]["root"]["Global"]["def_path"]
                .as_str()
                .is_some_and(|path| path.ends_with("RETURN_LOCK_A"))
                && edge["to"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("RETURN_LOCK_B"))
        });
    assert!(
        indirect_returned_guard_edge.is_some(),
        "missing indirect returned-guard edge"
    );

    let returned_guard_through_callback_wrapper =
        find_function(summary, "returned_guard_through_callback_wrapper");
    let wrapped_returned_guard_edge = returned_guard_through_callback_wrapper["lock_edges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|edge| {
            edge["from"]["class"]["root"]["Global"]["def_path"]
                .as_str()
                .is_some_and(|path| path.ends_with("RETURN_LOCK_A"))
                && edge["to"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("RETURN_LOCK_B"))
        });
    assert!(
        wrapped_returned_guard_edge.is_some(),
        "missing callback-wrapper returned-guard edge"
    );

    let returned_guard_through_callback_wrapper_irq_disabled = find_function(
        summary,
        "returned_guard_through_callback_wrapper_irq_disabled",
    );
    let wrapped_returned_guard_irq_disabled_edge =
        returned_guard_through_callback_wrapper_irq_disabled["lock_edges"]
            .as_array()
            .unwrap()
            .iter()
            .find(|edge| {
                edge["context"].as_str() == Some("TaskIrqDisabled")
                    && edge["from"]["class"]["root"]["Global"]["def_path"]
                        .as_str()
                        .is_some_and(|path| path.ends_with("RETURN_LOCK_A"))
                    && edge["to"]["class"]["root"]["Global"]["def_path"]
                        .as_str()
                        .is_some_and(|path| path.ends_with("RETURN_LOCK_B"))
            });
    assert!(
        wrapped_returned_guard_irq_disabled_edge.is_some(),
        "missing irq-disabled callback-wrapper returned-guard edge"
    );

    let returned_guard_through_callback_wrapper_with_lock =
        find_function(summary, "returned_guard_through_callback_wrapper_with_lock");
    let wrapped_returned_guard_with_lock_edge =
        returned_guard_through_callback_wrapper_with_lock["lock_edges"]
            .as_array()
            .unwrap()
            .iter()
            .find(|edge| {
                edge["from"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("RETURN_LOCK_A"))
                    && edge["to"]["class"]["root"]["Global"]["def_path"]
                        .as_str()
                        .is_some_and(|path| path.ends_with("RETURN_LOCK_B"))
            });
    assert!(
        wrapped_returned_guard_with_lock_edge.is_some(),
        "missing callback-wrapper-with-lock returned-guard edge"
    );

    let returned_guard_through_callback_wrapper_with_lock_irq_disabled = find_function(
        summary,
        "returned_guard_through_callback_wrapper_with_lock_irq_disabled",
    );
    let wrapped_returned_guard_with_lock_irq_disabled_edge =
        returned_guard_through_callback_wrapper_with_lock_irq_disabled["lock_edges"]
            .as_array()
            .unwrap()
            .iter()
            .find(|edge| {
                edge["context"].as_str() == Some("TaskIrqDisabled")
                    && edge["from"]["class"]["root"]["Global"]["def_path"]
                        .as_str()
                        .is_some_and(|path| path.ends_with("RETURN_LOCK_A"))
                    && edge["to"]["class"]["root"]["Global"]["def_path"]
                        .as_str()
                        .is_some_and(|path| path.ends_with("RETURN_LOCK_B"))
            });
    assert!(
        wrapped_returned_guard_with_lock_irq_disabled_edge.is_some(),
        "missing irq-disabled callback-wrapper-with-lock returned-guard edge"
    );

    let returned_guard_through_callback_wrapper_with_lock_alias = find_function(
        summary,
        "returned_guard_through_callback_wrapper_with_lock_alias",
    );
    let wrapped_returned_guard_with_lock_alias_edge =
        returned_guard_through_callback_wrapper_with_lock_alias["lock_edges"]
            .as_array()
            .unwrap()
            .iter()
            .find(|edge| {
                edge["from"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("RETURN_LOCK_A"))
                    && edge["to"]["class"]["root"]["Global"]["def_path"]
                        .as_str()
                        .is_some_and(|path| path.ends_with("RETURN_LOCK_B"))
            });
    assert!(
        wrapped_returned_guard_with_lock_alias_edge.is_some(),
        "missing alias callback-wrapper-with-lock returned-guard edge"
    );

    let returned_guard_through_callback_wrapper_alias =
        find_function(summary, "returned_guard_through_callback_wrapper_alias");
    let wrapped_returned_guard_alias_edge =
        returned_guard_through_callback_wrapper_alias["lock_edges"]
            .as_array()
            .unwrap()
            .iter()
            .find(|edge| {
                edge["from"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("RETURN_LOCK_A"))
                    && edge["to"]["class"]["root"]["Global"]["def_path"]
                        .as_str()
                        .is_some_and(|path| path.ends_with("RETURN_LOCK_B"))
            });
    assert!(
        wrapped_returned_guard_alias_edge.is_some(),
        "missing alias callback-wrapper returned-guard edge"
    );

    let returned_guard_through_outer_callback_wrapper =
        find_function(summary, "returned_guard_through_outer_callback_wrapper");
    let outer_wrapped_returned_guard_edge =
        returned_guard_through_outer_callback_wrapper["lock_edges"]
            .as_array()
            .unwrap()
            .iter()
            .find(|edge| {
                edge["from"]["class"]["root"]["Global"]["def_path"]
                    .as_str()
                    .is_some_and(|path| path.ends_with("RETURN_LOCK_A"))
                    && edge["to"]["class"]["root"]["Global"]["def_path"]
                        .as_str()
                        .is_some_and(|path| path.ends_with("RETURN_LOCK_B"))
            });
    assert!(
        outer_wrapped_returned_guard_edge.is_some(),
        "missing outer callback-wrapper returned-guard edge"
    );
}

#[test]
fn reports_atomic_mode_violations() {
    let summary = fixture_summary();

    let atomic_mode_violations = summary["global_report"]["atomic_mode_violations"]
        .as_array()
        .unwrap();
    assert!(atomic_mode_violations.iter().any(|report| {
        report["function"]
            .as_str()
            .is_some_and(|function| function.ends_with("atomic_spin_then_mutex"))
            && report["held_lock"]["primitive"].as_str() == Some("SpinLock")
            && report["sleeping_lock"]["primitive"].as_str() == Some("Mutex")
    }));
    assert!(atomic_mode_violations.iter().any(|report| {
        report["function"]
            .as_str()
            .is_some_and(|function| function.ends_with("atomic_rwlock_then_rwmutex"))
            && report["held_lock"]["primitive"].as_str() == Some("RwLock")
            && report["held_lock"]["acquire"].as_str() == Some("read")
            && report["sleeping_lock"]["primitive"].as_str() == Some("RwMutex")
            && report["sleeping_lock"]["acquire"].as_str() == Some("write")
    }));
    assert!(atomic_mode_violations.iter().any(|report| {
        report["function"]
            .as_str()
            .is_some_and(|function| function.ends_with("atomic_callsite_to_mutex_helper"))
            && report["held_lock"]["primitive"].as_str() == Some("SpinLock")
            && report["sleeping_lock"]["primitive"].as_str() == Some("Mutex")
    }));
}

#[test]
fn reports_cycles_and_explicit_drop_behavior() {
    let summary = fixture_summary();

    let cycle_steps = summary["global_report"]["cycles"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|cycle| {
            let steps = cycle["steps"].as_array().unwrap();
            let functions = steps
                .iter()
                .filter_map(|step| step["origin"]["function"].as_str())
                .collect::<Vec<_>>();
            (functions
                .iter()
                .any(|name| name.ends_with("deadlock_cycle_one"))
                && functions
                    .iter()
                    .any(|name| name.ends_with("deadlock_cycle_two")))
            .then_some(steps)
        })
        .expect("missing concrete global cycle for cycle fixture locks");
    let cycle_functions = cycle_steps
        .iter()
        .map(|step| step["origin"]["function"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(
        cycle_functions
            .iter()
            .any(|name| name.ends_with("deadlock_cycle_one"))
    );
    assert!(
        cycle_functions
            .iter()
            .any(|name| name.ends_with("deadlock_cycle_two"))
    );

    let helper_cycle_steps = summary["global_report"]["cycles"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|cycle| {
            let steps = cycle["steps"].as_array().unwrap();
            let functions = steps
                .iter()
                .filter_map(|step| step["origin"]["function"].as_str())
                .collect::<Vec<_>>();
            (functions
                .iter()
                .any(|name| name.ends_with("helper_cycle_one"))
                && functions
                    .iter()
                    .any(|name| name.ends_with("helper_cycle_two")))
            .then_some(steps)
        })
        .expect("missing propagated helper cycle");
    let helper_cycle_functions = helper_cycle_steps
        .iter()
        .map(|step| step["origin"]["function"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(
        helper_cycle_functions
            .iter()
            .any(|name| name.ends_with("helper_cycle_one"))
    );
    assert!(
        helper_cycle_functions
            .iter()
            .any(|name| name.ends_with("helper_cycle_two"))
    );

    let deadlock_cycle_one = find_function(summary, "deadlock_cycle_one");
    let from_root = &deadlock_cycle_one["lock_edges"][0]["from"]["class"]["root"]["Global"];
    let to_root = &deadlock_cycle_one["lock_edges"][0]["to"]["class"]["root"]["Global"];
    assert!(
        from_root["def_path"]
            .as_str()
            .is_some_and(|path| path.ends_with("CYCLE_LOCK_A"))
    );
    assert!(
        to_root["def_path"]
            .as_str()
            .is_some_and(|path| path.ends_with("CYCLE_LOCK_B"))
    );

    let helper_cycle_one = find_function(summary, "helper_cycle_one");
    let helper_edge_to = &helper_cycle_one["lock_edges"][0]["to"]["class"]["root"]["Global"];
    assert!(
        helper_edge_to["def_path"]
            .as_str()
            .is_some_and(|path| path.ends_with("HELPER_LOCK_B"))
    );

    let branch_cycle = find_function(summary, "branch_dependent_cycle");
    let branch_edges = branch_cycle["lock_edges"].as_array().unwrap();
    assert!(branch_edges.iter().any(|edge| {
        edge["from"]["class"]["root"]["Global"]["def_path"]
            .as_str()
            .is_some_and(|path| path.ends_with("BRANCH_LOCK_A"))
            && edge["to"]["class"]["root"]["Global"]["def_path"]
                .as_str()
                .is_some_and(|path| path.ends_with("BRANCH_LOCK_B"))
    }));
    assert!(branch_edges.iter().any(|edge| {
        edge["from"]["class"]["root"]["Global"]["def_path"]
            .as_str()
            .is_some_and(|path| path.ends_with("BRANCH_LOCK_B"))
            && edge["to"]["class"]["root"]["Global"]["def_path"]
                .as_str()
                .is_some_and(|path| path.ends_with("BRANCH_LOCK_A"))
    }));
    let branch_cycle_steps = summary["global_report"]["cycles"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|cycle| {
            let steps = cycle["steps"].as_array().unwrap();
            steps
                .iter()
                .any(|step| {
                    step["origin"]["function"]
                        .as_str()
                        .is_some_and(|function| function.ends_with("branch_dependent_cycle"))
                })
                .then_some(steps)
        })
        .expect("missing branch-dependent cycle");
    assert!(branch_cycle_steps.iter().all(|step| {
        step["origin"]["function"]
            .as_str()
            .is_some_and(|function| function.ends_with("branch_dependent_cycle"))
    }));

    let explicit_drop_then_relock = find_function(summary, "explicit_drop_then_relock");
    let explicit_drop_events = explicit_drop_then_relock["lock_events"].as_array().unwrap();
    assert_eq!(explicit_drop_events.len(), 4);
    assert_eq!(explicit_drop_events[0]["kind"].as_str(), Some("acquire"));
    assert_eq!(explicit_drop_events[1]["kind"].as_str(), Some("release"));
    assert_eq!(explicit_drop_events[2]["kind"].as_str(), Some("acquire"));
    assert_eq!(explicit_drop_events[3]["kind"].as_str(), Some("release"));
    assert!(
        explicit_drop_then_relock["lock_edges"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    let aa_deadlocks = summary["global_report"]["aa_deadlocks"].as_array().unwrap();
    assert!(aa_deadlocks.iter().any(|report| {
        report["kind"].as_str() == Some("self_lock")
            && report["sites"][0]["function"]
                .as_str()
                .is_some_and(|function| function.ends_with("aa_self_deadlock"))
    }));
    assert!(
        aa_deadlocks
            .iter()
            .any(|report| report["kind"].as_str() == Some("irq_reentry"))
    );
}

#[test]
fn preserves_lock_class_identity_shapes() {
    let summary = fixture_summary();

    let deadlock_args_one_class = first_lock_class(summary, "deadlock_args_one");
    let arg_root = root_variant(deadlock_args_one_class, "FnArg");
    assert_eq!(arg_root["index"].as_u64(), Some(1));
    assert!(
        arg_root["fn_def_path"]
            .as_str()
            .is_some_and(|path| path.ends_with("deadlock_args_one"))
    );

    let lock_inner_class = first_lock_class(summary, "lock_inner");
    let receiver_root = root_variant(lock_inner_class, "ReceiverArg");
    assert!(
        receiver_root["method_def_path"]
            .as_str()
            .is_some_and(|path| path.contains("lock_inner"))
    );
    assert!(
        receiver_root["self_ty"]
            .as_str()
            .is_some_and(|ty| ty.contains("LockHolder"))
    );
    let receiver_field = &find_projection(lock_inner_class, "Field")["Field"];
    assert_eq!(receiver_field["field_name"].as_str(), Some("inner"));
    assert!(
        receiver_field["owner_ty"]
            .as_str()
            .is_some_and(|ty| ty.contains("LockHolder"))
    );

    let fn_arg_alias_class = first_lock_class(summary, "fn_arg_alias_path");
    let fn_arg_alias_root = root_variant(fn_arg_alias_class, "FnArg");
    assert_eq!(fn_arg_alias_root["index"].as_u64(), Some(1));
    assert!(
        fn_arg_alias_root["fn_def_path"]
            .as_str()
            .is_some_and(|path| path.ends_with("fn_arg_alias_path"))
    );

    let global_ref_alias_class = first_lock_class(summary, "global_ref_alias_path");
    let global_ref_alias_root = root_variant(global_ref_alias_class, "Global");
    assert!(
        global_ref_alias_root["def_path"]
            .as_str()
            .is_some_and(|path| path.ends_with("LOCK_A"))
    );

    let aggregate_local_class = first_lock_class(summary, "aggregate_local_path");
    let aggregate_local_root = root_variant(aggregate_local_class, "Local");
    assert!(
        aggregate_local_root["fn_def_path"]
            .as_str()
            .is_some_and(|path| path.ends_with("aggregate_local_path"))
    );
    assert!(
        aggregate_local_root["origin"]["AggregateTemp"]["ty"]
            .as_str()
            .is_some_and(|ty| ty.contains("TempHolder"))
    );
    let aggregate_field = &find_projection(aggregate_local_class, "Field")["Field"];
    assert_eq!(aggregate_field["field_name"].as_str(), Some("right"));
    assert!(
        aggregate_field["owner_ty"]
            .as_str()
            .is_some_and(|ty| ty.contains("TempHolder"))
    );

    let downcast_class = first_lock_class(summary, "downcast_projection_path");
    let downcast_root = root_variant(downcast_class, "Local");
    assert!(
        downcast_root["origin"]["AggregateTemp"]["ty"]
            .as_str()
            .is_some_and(|ty| ty.contains("VariantLock"))
    );

    let constant_index_class = first_lock_class(summary, "constant_index_projection_path");
    assert!(find_projection(constant_index_class, "Index").is_object());

    let variable_index_class = first_lock_class(summary, "variable_index_projection_path");
    let variable_index_root = root_variant(variable_index_class, "Local");
    assert!(
        variable_index_root["origin"]["AggregateTemp"]["ty"]
            .as_str()
            .is_some_and(|ty| ty.contains("[ostd::sync::SpinLock<u32>; 2]"))
    );
    assert!(find_projection(variable_index_class, "Deref").is_object());
}

#[test]
fn reports_irq_violation_classes() {
    let summary = fixture_summary();

    let single_lock_violations = summary["global_report"]["single_lock_irq_violations"]
        .as_array()
        .unwrap();
    assert!(single_lock_violations.iter().any(|report| {
        report["kind"].as_str() == Some("hardirq_safe_vs_unsafe")
            && report["safe_site"]["function"]
                .as_str()
                .is_some_and(|function| function.ends_with("irq_conflict_worker"))
            && report["unsafe_site"]["function"]
                .as_str()
                .is_some_and(|function| function.ends_with("irq_conflict_worker"))
    }));
    assert!(single_lock_violations.iter().any(|report| {
        report["kind"].as_str() == Some("hardirq_safe_vs_unsafe")
            && report["safe_site"]["function"]
                .as_str()
                .is_some_and(|function| function.ends_with("dependency_target_worker"))
            && report["unsafe_site"]["function"]
                .as_str()
                .is_some_and(|function| function.ends_with("dependency_target_worker"))
    }));
    assert!(single_lock_violations.iter().any(|report| {
        report["kind"].as_str() == Some("softirq_safe_vs_unsafe")
            && report["safe_site"]["function"]
                .as_str()
                .is_some_and(|function| function.ends_with("configured_softirq_worker"))
            && report["unsafe_site"]["function"]
                .as_str()
                .is_some_and(|function| function.ends_with("configured_softirq_worker"))
    }));

    assert!(!single_lock_violations.iter().any(|report| {
        report["safe_site"]["function"]
            .as_str()
            .is_some_and(|function| function.ends_with("shared_irq_disabled_worker"))
            || report["unsafe_site"]["function"]
                .as_str()
                .is_some_and(|function| function.ends_with("shared_irq_disabled_worker"))
    }));

    let dependency_violations = summary["global_report"]["irq_dependency_violations"]
        .as_array()
        .unwrap();
    assert!(dependency_violations.iter().any(|report| {
        report["kind"].as_str() == Some("hardirq_safe_to_unsafe")
            && report["witness_edge_origin"]["function"]
                .as_str()
                .is_some_and(|function| function.ends_with("task_dependency_edge"))
            && report["from_safe_site"]["function"]
                .as_str()
                .is_some_and(|function| {
                    function.ends_with("task_dependency_edge")
                        || function.contains("register_top_half::{closure#0}")
                })
            && report["to_unsafe_site"]["function"]
                .as_str()
                .is_some_and(|function| function.ends_with("dependency_target_worker"))
    }));
}
