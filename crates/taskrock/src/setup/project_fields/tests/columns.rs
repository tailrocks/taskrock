//! Tests of the columns and of the dry run without a project.

use serde_json::json;

use super::{ROUTING, fields, found, run_step, sent, wanted};
use crate::config::ProjectKind;
use crate::gh::FakeGh;

const COMPLETE: [&str; 4] = ["Todo", "In Progress", "Done", "Triage"];

#[test]
fn a_missing_column_is_added_from_the_org_field() {
    let present = &ROUTING[..7];
    let org = json!([{"id": 8, "name": "Review effort", "node_id": "IF_8"}]);
    let gh = FakeGh::new()
        .ok("graphql", &fields(&COMPLETE, present))
        .ok("issue-fields", &org.to_string())
        .ok("graphql", r#"{"data":{}}"#);
    let list = [(wanted(ProjectKind::Delivery), Some(found()))];
    let (result, lines) = run_step(&gh, &list, false);
    result.expect("step");
    assert_eq!(
        lines.last().expect("line"),
        "create column taskrock Review effort"
    );
    assert_eq!(
        lines
            .iter()
            .filter(|line| line.starts_with("exists"))
            .count(),
        8
    );
    let (query, variables) = sent(&gh, 2);
    assert!(query.contains("createProjectV2IssueField(input:{projectId:$p,issueFieldId:$i})"));
    assert_eq!(variables, json!({"p": "P_3", "i": "IF_8"}));
}

#[test]
fn a_portfolio_project_wants_the_two_dates() {
    let org = json!([{"id": 1, "name": "Target date", "node_id": "IF_T"}]);
    let gh = FakeGh::new()
        .ok("graphql", &fields(&COMPLETE, &["Start date"]))
        .ok("issue-fields", &org.to_string())
        .ok("graphql", r#"{"data":{}}"#);
    let list = [(wanted(ProjectKind::Portfolio), Some(found()))];
    let (result, lines) = run_step(&gh, &list, false);
    result.expect("step");
    assert_eq!(
        lines,
        [
            "exists status taskrock",
            "exists column taskrock Start date",
            "create column taskrock Target date",
        ]
    );
    assert_eq!(sent(&gh, 2).1["i"], "IF_T");
}

#[test]
fn a_project_that_is_none_says_create_for_everything() {
    let gh = FakeGh::new();
    let list = [
        (wanted(ProjectKind::Delivery), None),
        (wanted(ProjectKind::Portfolio), None),
    ];
    let (result, lines) = run_step(&gh, &list, true);
    result.expect("step");
    assert_eq!(gh.calls().len(), 0);
    assert_eq!(lines.len(), 1 + 8 + 1 + 2);
    assert_eq!(lines[0], "create status taskrock");
    assert_eq!(lines[1], "create column taskrock Agent");
    assert_eq!(lines[11], "create column taskrock Target date");
}

#[test]
fn a_column_the_organization_lacks_is_an_error() {
    let gh = FakeGh::new()
        .ok("graphql", &fields(&COMPLETE, &ROUTING[..7]))
        .ok("issue-fields", "[]");
    let list = [(wanted(ProjectKind::Delivery), Some(found()))];
    let error = run_step(&gh, &list, false).0.expect_err("no org field");
    assert_eq!(
        error.to_string(),
        "the organization has no issue field Review effort"
    );
}

#[test]
fn a_dry_run_reads_no_org_field() {
    let gh = FakeGh::new().ok("graphql", &fields(&COMPLETE, &ROUTING[..7]));
    let list = [(wanted(ProjectKind::Delivery), Some(found()))];
    let (result, lines) = run_step(&gh, &list, true);
    result.expect("step");
    assert_eq!(
        lines.last().expect("line"),
        "create column taskrock Review effort"
    );
    assert_eq!(gh.calls().len(), 1);
}
