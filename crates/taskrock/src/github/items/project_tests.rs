//! Tests of `project_of`: the delivery project of an issue.

use crate::config::ProjectKind;
use crate::error::Error;
use crate::gh::FakeGh;
use crate::github::items::project_of;
use crate::link::{IssueLink, ProjectLink};
use crate::projects::{Marker, marker_line};

fn issue() -> IssueLink {
    "https://github.com/tailrocks/taskrock/issues/12"
        .parse()
        .expect("link")
}

fn project(number: u64) -> ProjectLink {
    ProjectLink {
        owner: "tailrocks".to_owned(),
        number,
    }
}

fn node(number: u64, kind: Option<ProjectKind>) -> serde_json::Value {
    let readme = kind.map(|kind| {
        let marker = Marker {
            key: format!("k{number}"),
            kind,
        };
        format!("About.\n\n{}", marker_line(&marker))
    });
    serde_json::json!({"project": {"number": number, "readme": readme, "owner": {"login": "tailrocks"}}})
}

fn answer(nodes: &[serde_json::Value]) -> FakeGh {
    let body =
        serde_json::json!({"data": {"repository": {"issue": {"projectItems": {"nodes": nodes}}}}});
    FakeGh::new().ok("graphql", &body.to_string())
}

fn message(nodes: &[serde_json::Value]) -> String {
    match project_of(&answer(nodes), &issue()).expect_err("no single project") {
        Error::Project(message) => message,
        other => panic!("not a project error: {other}"),
    }
}

#[test]
fn project_of_returns_project_three() {
    let gh = answer(&[node(3, None)]);
    assert_eq!(project_of(&gh, &issue()).expect("project"), project(3));
}

#[test]
fn project_of_drops_the_portfolio_project() {
    let nodes = [
        node(4, Some(ProjectKind::Portfolio)),
        node(3, Some(ProjectKind::Delivery)),
    ];
    let gh = answer(&nodes);
    assert_eq!(project_of(&gh, &issue()).expect("project"), project(3));
}

#[test]
fn project_with_no_marker_counts_as_delivery() {
    let nodes = [node(4, Some(ProjectKind::Portfolio)), node(3, None)];
    let gh = answer(&nodes);
    assert_eq!(project_of(&gh, &issue()).expect("project"), project(3));
}

#[test]
fn project_of_rejects_no_delivery_project() {
    let url = issue().url();
    let want = format!("the issue is in no delivery project: {url}");
    assert_eq!(message(&[]), want);
    assert_eq!(message(&[node(4, Some(ProjectKind::Portfolio))]), want);
}

#[test]
fn project_of_rejects_two_delivery_projects() {
    let nodes = [node(3, None), node(5, Some(ProjectKind::Delivery))];
    let want = format!(
        "the issue is in more than one delivery project: {}",
        issue().url()
    );
    assert_eq!(message(&nodes), want);
}

#[test]
fn project_of_sends_the_repository_query_with_a_numeric_number() {
    let gh = answer(&[node(3, None)]);
    project_of(&gh, &issue()).expect("project");
    let calls = gh.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "api graphql --input -");
    let sent: serde_json::Value =
        serde_json::from_str(calls[0].1.as_deref().expect("input")).expect("json");
    let query = sent["query"].as_str().expect("query");
    assert!(query.contains("repository(owner:$o,name:$r){issue(number:$n)"));
    let want = serde_json::json!({"o": "tailrocks", "r": "taskrock", "n": 12});
    assert_eq!(sent["variables"], want);
}
