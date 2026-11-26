//! Tests of the projects step.

use std::path::PathBuf;

use serde_json::{Value, json};

use super::projects;

mod fixes;
use crate::config::{self, Config};
use crate::error::Error;
use crate::gh::FakeGh;
use crate::projects::OrgProject;
use crate::setup::Run;

pub(super) const DELIVERY: &str = "<!-- taskrock:project key=taskrock kind=delivery -->";
const PORTFOLIO: &str = "<!-- taskrock:project key=roadmap kind=portfolio -->";

/// The config of `config-good.toml` with only its first project: `taskrock`,
/// number 3.
pub(super) fn one() -> Config {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/config-good.toml");
    let mut config = config::parse::read(&file).expect("good config");
    config.projects.truncate(1);
    config
}

pub(super) fn node(number: u64, title: &str, readme: &str, public: bool) -> Value {
    json!({"id": format!("P_{number}"), "number": number, "title": title, "readme": readme, "public": public, "closed": false})
}

pub(super) fn listing(nodes: &[Value]) -> String {
    json!({"data": {"organization": {"projectsV2": {"nodes": nodes}}}}).to_string()
}

type Outcome = (
    Result<Vec<(config::Project, Option<OrgProject>)>, Error>,
    Vec<String>,
);

pub(super) fn run_step(gh: &FakeGh, config: &Config, dry_run: bool) -> Outcome {
    let mut run = Run {
        gh,
        owner: "acme".to_owned(),
        dry_run,
        yes: false,
        lines: Vec::new(),
    };
    let result = projects(&mut run, config);
    (result, run.lines)
}

/// The GraphQL query and variables of call `index`.
pub(in crate::setup) fn sent(gh: &FakeGh, index: usize) -> (String, Value) {
    let input = gh.calls()[index].1.clone().expect("input");
    let body: Value = serde_json::from_str(&input).expect("json");
    (
        body["query"].as_str().expect("query").to_owned(),
        body["variables"].clone(),
    )
}

#[test]
fn a_project_found_by_marker_exists() {
    let found = node(9, "taskrock", DELIVERY, false);
    let gh = FakeGh::new().ok("projectsV2", &listing(&[found]));
    let (result, lines) = run_step(&gh, &one(), false);
    let projects = result.expect("step");
    assert_eq!(lines, ["exists project taskrock"]);
    assert_eq!(gh.calls().len(), 1);
    assert_eq!(projects[0].1.as_ref().expect("project").number, 9);
}

#[test]
fn a_project_found_by_number_is_marked() {
    let found = node(3, "taskrock", "About it.", false);
    let gh = FakeGh::new()
        .ok("projectsV2", &listing(&[found]))
        .ok("graphql", r#"{"data":{}}"#);
    let (result, lines) = run_step(&gh, &one(), false);
    result.expect("step");
    assert_eq!(lines, ["update project taskrock (marker)"]);
    let (query, variables) = sent(&gh, 1);
    assert_eq!(variables["p"], "P_3");
    assert!(
        query.contains(
            r#"readme:"About it.\n\n<!-- taskrock:project key=taskrock kind=delivery -->""#
        )
    );
}

#[test]
fn a_project_with_the_marker_of_another_key_is_refused() {
    let found = node(3, "taskrock", PORTFOLIO, false);
    let gh = FakeGh::new().ok("projectsV2", &listing(&[found]));
    let (result, lines) = run_step(&gh, &one(), false);
    let error = result.expect_err("other marker");
    assert_eq!(error.to_string(), "project 3 has the marker of roadmap");
    assert_eq!(lines.len(), 0);
    assert_eq!(gh.calls().len(), 1);
}

#[test]
fn a_missing_project_is_created_then_marked() {
    let gh = FakeGh::new()
        .ok("projectsV2", &listing(&[]))
        .ok("graphql", r#"{"data":{"organization":{"id":"O_1"}}}"#)
        .ok(
            "graphql",
            r#"{"data":{"createProjectV2":{"projectV2":{"id":"P_7","number":7}}}}"#,
        )
        .ok("graphql", r#"{"data":{}}"#);
    let (result, lines) = run_step(&gh, &one(), false);
    let projects = result.expect("step");
    assert_eq!(
        lines,
        [
            "create project taskrock",
            "update project taskrock (marker)"
        ]
    );
    let (query, variables) = sent(&gh, 2);
    assert!(query.contains("createProjectV2(input:{ownerId:$o,title:$t})"));
    assert_eq!(
        (variables["o"].as_str(), variables["t"].as_str()),
        (Some("O_1"), Some("taskrock"))
    );
    let (query, variables) = sent(&gh, 3);
    assert_eq!(variables["p"], "P_7");
    assert!(query.contains(DELIVERY));
    let made = projects[0].1.as_ref().expect("project");
    assert_eq!((made.number, made.readme.as_str()), (7, DELIVERY));
}
