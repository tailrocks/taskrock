//! Tests of the repositories step.

use serde_json::json;

use super::repos;
use crate::gh::FakeGh;
use crate::setup::projects::tests::sent;
use crate::setup::tests::testing::{project, step};

fn linked(names: &[&str]) -> String {
    let nodes: Vec<_> = names.iter().map(|name| json!({"name": name})).collect();
    json!({"data": {"node": {"repositories": {"nodes": nodes}}}}).to_string()
}

#[test]
fn a_missing_link_is_created_with_the_node_id() {
    let gh = FakeGh::new()
        .ok("graphql", &linked(&["taskrock"]))
        .ok("repos/acme/taskrock-plugins", r#"{"node_id":"R_2"}"#)
        .ok("graphql", r#"{"data":{}}"#);
    let list = [project(&["taskrock", "taskrock-plugins"], Vec::new(), true)];
    let (result, lines) = step(&gh, false, |run| repos(run, &list));
    result.expect("step");
    assert_eq!(
        lines,
        [
            "exists link taskrock acme/taskrock",
            "create link taskrock acme/taskrock-plugins",
        ]
    );
    let (query, variables) = sent(&gh, 2);
    assert!(query.contains("linkProjectV2ToRepository"));
    assert_eq!(variables, json!({"p": "P_3", "r": "R_2"}));
}

#[test]
fn an_extra_link_is_noted_and_kept() {
    let gh = FakeGh::new().ok("graphql", &linked(&["taskrock", "old"]));
    let list = [project(&["taskrock"], Vec::new(), true)];
    let (result, lines) = step(&gh, false, |run| repos(run, &list));
    result.expect("step");
    assert_eq!(
        lines,
        [
            "exists link taskrock acme/taskrock",
            "extra link taskrock acme/old"
        ]
    );
    assert_eq!(gh.calls().len(), 1);
}

#[test]
fn a_dry_run_reads_and_writes_nothing_more() {
    let gh = FakeGh::new().ok("graphql", &linked(&[]));
    let list = [project(&["taskrock"], Vec::new(), true)];
    let (result, lines) = step(&gh, true, |run| repos(run, &list));
    result.expect("step");
    assert_eq!(lines, ["create link taskrock acme/taskrock"]);
    assert_eq!(gh.calls().len(), 1);
}

#[test]
fn a_project_that_is_none_says_create_with_no_read() {
    let gh = FakeGh::new();
    let list = [project(
        &["taskrock", "taskrock-plugins"],
        Vec::new(),
        false,
    )];
    let (result, lines) = step(&gh, true, |run| repos(run, &list));
    result.expect("step");
    assert_eq!(
        lines,
        [
            "create link taskrock acme/taskrock",
            "create link taskrock acme/taskrock-plugins",
        ]
    );
    assert_eq!(gh.calls().len(), 0);
}

#[test]
fn a_repository_without_node_id_is_an_error() {
    let gh = FakeGh::new()
        .ok("graphql", &linked(&[]))
        .ok("repos/acme/taskrock", "{}");
    let list = [project(&["taskrock"], Vec::new(), true)];
    let error = step(&gh, false, |run| repos(run, &list))
        .0
        .expect_err("no node_id");
    assert!(error.to_string().contains("no node_id"));
}
