//! Tests of the workflows step.

use serde_json::{Value, json};

mod support;
use self::support::{all_on, answer, off, only_exists, run_step};
use crate::config::Workflows;
use crate::gh::FakeGh;

#[test]
fn every_workflow_on_and_present_exists() {
    let gh = FakeGh::new().ok("graphql", &answer(&all_on(), &[]));
    let (result, lines) = run_step(&gh, Workflows::default(), true, false, false);
    result.expect("step");
    assert!(only_exists(&lines));
    assert_eq!(lines[0], "exists workflow Auto-add sub-issues to project");
    assert_eq!(gh.calls().len(), 1);
}

#[test]
fn a_missing_workflow_set_to_true_stops_the_step() {
    let gh = FakeGh::new().ok("graphql", &answer(&all_on()[..5], &[]));
    let (result, _) = run_step(&gh, Workflows::default(), true, true, false);
    let error = result.expect_err("missing");
    assert_eq!(
        error.to_string(),
        "project taskrock: the workflow Auto-close issue is off and the API cannot turn it on"
    );
}

#[test]
fn a_workflow_set_to_true_but_not_enabled_stops_the_step() {
    let mut present = all_on();
    present[2].1 = false;
    let gh = FakeGh::new().ok("graphql", &answer(&present, &[]));
    let (result, _) = run_step(&gh, Workflows::default(), true, true, false);
    let message = result.expect_err("off").to_string();
    assert!(message.contains("the workflow Item closed is off"));
}

#[test]
fn a_workflow_set_to_false_is_deleted_with_yes() {
    let set = off(|w| w.item_closed = false);
    let gh = FakeGh::new()
        .ok("graphql", &answer(&all_on(), &[]))
        .ok("graphql", r#"{"data":{}}"#);
    let (result, lines) = run_step(&gh, set, true, true, false);
    result.expect("step");
    assert_eq!(lines[2], "delete workflow Item closed");
    let calls = gh.calls();
    assert_eq!(calls.len(), 2);
    let body: Value = serde_json::from_str(calls[1].1.as_deref().expect("input")).expect("json");
    assert!(
        body["query"]
            .as_str()
            .expect("query")
            .contains("deleteProjectV2Workflow")
    );
    assert_eq!(body["variables"], json!({"w": "W_Item closed"}));
}

#[test]
fn a_workflow_set_to_false_is_kept_without_yes() {
    let set = off(|w| w.pr_merged = false);
    let gh = FakeGh::new().ok("graphql", &answer(&all_on(), &[]));
    let (result, lines) = run_step(&gh, set, true, false, false);
    result.expect("step");
    assert_eq!(
        lines[4],
        "workflow Pull request merged is set to false. Run again with --yes to delete it for good"
    );
    assert_eq!(gh.calls().len(), 1);
}

#[test]
fn a_deleted_workflow_set_to_false_exists() {
    let set = off(|w| w.pr_linked = false);
    let present: Vec<_> = all_on()
        .into_iter()
        .filter(|(n, _)| !n.contains("linked"))
        .collect();
    let gh = FakeGh::new().ok("graphql", &answer(&present, &[]));
    let (result, lines) = run_step(&gh, set, true, true, false);
    result.expect("step");
    assert_eq!(
        lines[3],
        "exists workflow Pull request linked to issue (deleted)"
    );
    assert_eq!(gh.calls().len(), 1);
}

#[test]
fn a_new_project_has_the_six_workflows_and_reads_nothing() {
    let gh = FakeGh::new();
    let (result, lines) = run_step(&gh, Workflows::default(), false, false, true);
    result.expect("step");
    assert!(only_exists(&lines));
    assert_eq!(gh.calls().len(), 0);
}

#[test]
fn a_dry_run_says_the_delete_and_sends_nothing() {
    let set = off(|w| w.auto_add_sub_issues = false);
    let gh = FakeGh::new().ok("graphql", &answer(&all_on(), &[]));
    let (result, lines) = run_step(&gh, set, true, true, true);
    result.expect("step");
    assert_eq!(lines[0], "delete workflow Auto-add sub-issues to project");
    assert_eq!(gh.calls().len(), 1);
}

#[test]
fn a_board_with_no_group_by_or_status_is_fine() {
    let views: [(&str, &str, &[&str]); 3] = [
        ("Board", "BOARD_LAYOUT", &[]),
        ("Flow", "BOARD_LAYOUT", &["Status"]),
        ("Table", "TABLE_LAYOUT", &["Agent"]),
    ];
    let gh = FakeGh::new().ok("graphql", &answer(&all_on(), &views));
    let (result, lines) = run_step(&gh, Workflows::default(), true, false, false);
    result.expect("step");
    assert!(only_exists(&lines));
}

#[test]
fn a_board_grouped_by_another_field_is_reported() {
    let views: [(&str, &str, &[&str]); 1] = [("Board", "BOARD_LAYOUT", &["Agent", "Model"])];
    let gh = FakeGh::new().ok("graphql", &answer(&all_on(), &views));
    let (result, lines) = run_step(&gh, Workflows::default(), true, false, false);
    result.expect("step");
    assert_eq!(lines.len(), 7);
    assert_eq!(
        lines[6],
        "view Board groups by Agent, Model, not Status. The API cannot change it"
    );
}
