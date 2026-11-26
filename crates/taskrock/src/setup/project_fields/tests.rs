//! Tests of the project fields step.

use serde_json::{Value, json};

use super::project_fields;
use crate::config::{Project, ProjectKind};
use crate::error::Error;
use crate::gh::FakeGh;
use crate::projects::OrgProject;
use crate::setup::Run;
use crate::setup::projects::tests::sent;

mod columns;

pub(super) fn wanted(kind: ProjectKind) -> Project {
    Project {
        key: "taskrock".to_owned(),
        title: "taskrock".to_owned(),
        kind,
        number: None,
        repos: Vec::new(),
        workflows: crate::config::Workflows::default(),
        milestones: Vec::new(),
    }
}

pub(super) fn found() -> OrgProject {
    OrgProject {
        id: "P_3".to_owned(),
        number: 3,
        title: "taskrock".to_owned(),
        readme: String::new(),
        public: false,
        marker: None,
    }
}

pub(super) fn run_step(
    gh: &FakeGh,
    list: &[(Project, Option<OrgProject>)],
    dry_run: bool,
) -> (Result<(), Error>, Vec<String>) {
    let mut run = Run {
        gh,
        owner: "acme".to_owned(),
        dry_run,
        yes: false,
        lines: Vec::new(),
    };
    let result = project_fields(&mut run, list);
    (result, run.lines)
}

/// The Status field with these options, then a field for each name.
pub(super) fn fields(options: &[&str], names: &[&str]) -> String {
    let options: Vec<Value> = (1..)
        .zip(options)
        .map(|(n, name)| json!({"id": format!("O_{n}"), "name": name, "color": "BLUE", "description": format!("about {name}")}))
        .collect();
    let mut nodes = vec![json!({"id": "F_status", "name": "Status", "options": options})];
    nodes.extend(
        names
            .iter()
            .map(|name| json!({"id": format!("F_{name}"), "name": name})),
    );
    json!({"data": {"node": {"fields": {"nodes": nodes}}}}).to_string()
}

pub(super) const ROUTING: [&str; 8] = [
    "Agent",
    "Review agent",
    "Runtime",
    "Review runtime",
    "Model",
    "Review model",
    "Effort",
    "Review effort",
];

#[test]
fn triage_is_added_with_the_existing_ids() {
    let gh = FakeGh::new()
        .ok(
            "graphql",
            &fields(&["Todo", "In Progress", "Done"], &ROUTING),
        )
        .ok("graphql", r#"{"data":{}}"#);
    let list = [(wanted(ProjectKind::Delivery), Some(found()))];
    let (result, lines) = run_step(&gh, &list, false);
    result.expect("step");
    assert_eq!(lines[0], "update status taskrock (add Triage)");
    assert_eq!(lines.len(), 9);
    let (query, variables) = sent(&gh, 1);
    assert!(query.contains("updateProjectV2Field(input:{fieldId:$f,singleSelectOptions:$o})"));
    assert_eq!(variables["f"], "F_status");
    assert_eq!(
        variables["o"],
        json!([
            {"id": "O_1", "name": "Todo", "color": "BLUE", "description": "about Todo"},
            {"id": "O_2", "name": "In Progress", "color": "BLUE", "description": "about In Progress"},
            {"id": "O_3", "name": "Done", "color": "BLUE", "description": "about Done"},
            {"name": "Triage", "color": "GRAY", "description": "A reported issue that waits for triage"},
        ])
    );
}

#[test]
fn a_status_with_all_options_exists() {
    let status = ["Todo", "In Progress", "Done", "Triage"];
    let gh = FakeGh::new().ok("graphql", &fields(&status, &ROUTING));
    let list = [(wanted(ProjectKind::Delivery), Some(found()))];
    let (result, lines) = run_step(&gh, &list, false);
    result.expect("step");
    assert_eq!(lines[0], "exists status taskrock");
    assert_eq!(gh.calls().len(), 1);
}

#[test]
fn a_dry_run_sends_no_status_update() {
    let gh = FakeGh::new().ok("graphql", &fields(&["Todo"], &ROUTING));
    let list = [(wanted(ProjectKind::Delivery), Some(found()))];
    let (result, lines) = run_step(&gh, &list, true);
    result.expect("step");
    assert_eq!(
        lines[0],
        "update status taskrock (add Triage, In Progress, Done)"
    );
    assert_eq!(gh.calls().len(), 1);
}

#[test]
fn a_project_without_status_is_an_error() {
    let body = json!({"data": {"node": {"fields": {"nodes": [{"id": "F_1", "name": "Title"}]}}}});
    let gh = FakeGh::new().ok("graphql", &body.to_string());
    let list = [(wanted(ProjectKind::Delivery), Some(found()))];
    let error = run_step(&gh, &list, false).0.expect_err("no status");
    assert_eq!(error.to_string(), "project taskrock has no Status field");
}
