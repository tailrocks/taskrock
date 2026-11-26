//! Tests of `set_statuses` on a re-run, with issues found from an earlier run.

use serde_json::{Value, json};

use super::{item, run_with};

/// Like [`item`], with the field Agent set.
fn routed_item(number: u64, status: Option<&str>) -> Value {
    let mut value = item(number, status);
    value["content"]["issue_field_values"] = json!([{"issue_field_name":"Agent",
        "single_select_option":{"name":"tailrocks-systems-engineer"}}]);
    value
}

#[test]
fn existing_bug_and_task_keep_their_status() {
    let items = [item(12, Some("Todo")), routed_item(13, Some("Todo"))];
    let (lines, gh) = run_with(&["T01", "B1"], &items, &items);
    assert!(lines.is_empty(), "nothing to change: {lines:?}");
    assert!(!gh.calls().iter().any(|(call, _)| call.contains("PATCH")));
}

#[test]
fn existing_bug_at_todo_without_routing_goes_to_triage() {
    let items = [item(12, Some("Todo")), item(13, Some("Todo"))];
    let (lines, _) = run_with(&["T01", "B1"], &items, &items);
    assert_eq!(lines, ["status: #13 Triage", "status: #13 Triage"]);
}

#[test]
fn existing_bug_at_todo_with_an_agent_keeps_its_status() {
    let items = [item(12, Some("Todo")), routed_item(13, Some("Todo"))];
    let (lines, gh) = run_with(&["T01", "B1"], &items, &items);
    assert!(lines.is_empty(), "nothing to change: {lines:?}");
    assert!(!gh.calls().iter().any(|(call, _)| call.contains("PATCH")));
}

#[test]
fn new_bug_goes_to_triage_next_to_an_existing_task() {
    let first = [item(12, None), item(13, Some("Todo"))];
    let (lines, _) = run_with(&["T01"], &first, &first);
    assert_eq!(
        lines,
        [
            "status: #13 Triage",
            "status: #12 Todo",
            "status: #13 Triage"
        ]
    );
}

#[test]
fn new_task_goes_to_todo_next_to_an_existing_triaged_bug() {
    let items = [item(12, None), item(13, Some("Triage"))];
    let (lines, _) = run_with(&["B1"], &items, &items);
    assert_eq!(lines, ["status: #12 Todo"]);
}

#[test]
fn rerun_after_a_partial_failure_sets_both_missing_statuses() {
    let none = [item(12, None), item(13, None)];
    let after = [item(12, Some("Todo")), item(13, Some("Triage"))];
    let (lines, _) = run_with(&["T01", "B1"], &none, &after);
    assert_eq!(lines, ["status: #13 Triage", "status: #12 Todo"]);
}
