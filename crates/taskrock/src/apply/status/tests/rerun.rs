//! Tests of `set_statuses` on a re-run, with issues found from an earlier run.

use super::{item, run_with};

#[test]
fn existing_bug_and_task_keep_their_status() {
    let items = [item(12, Some("Todo")), item(13, Some("Todo"))];
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
