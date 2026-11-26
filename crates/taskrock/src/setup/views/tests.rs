//! Tests of the views step.

use serde_json::{Value, json};

use super::{DELIVERY, PORTFOLIO, ViewSpec, views};
use crate::config::{Project, ProjectKind};
use crate::error::Error;
use crate::gh::FakeGh;
use crate::projects::OrgProject;
use crate::setup::Run;
use crate::setup::project_fields::tests::{found, wanted};

mod create;
mod fix;

/// The answer to any mutation: it holds the ID of a created view.
const WRITE: &str = r#"{"data":{"createProjectV2View":{"projectV2View":{"id":"V_new"}}}}"#;

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
    let result = views(&mut run, list);
    (result, run.lines)
}

pub(super) fn existing(kind: ProjectKind) -> Vec<(Project, Option<OrgProject>)> {
    vec![(wanted(kind), Some(found()))]
}

fn specs(kind: ProjectKind) -> &'static [ViewSpec] {
    match kind {
        ProjectKind::Delivery => DELIVERY,
        ProjectKind::Portfolio => PORTFOLIO,
    }
}

/// Every column of both tables: an ID is the name.
fn all_columns() -> Vec<&'static str> {
    let mut names: Vec<&str> = DELIVERY
        .iter()
        .chain(PORTFOLIO)
        .flat_map(|spec| spec.columns.iter().copied())
        .collect();
    names.extend(["Assignees", "Linked pull requests"]);
    names.sort_unstable();
    names.dedup();
    names
}

/// A view as GraphQL returns it. An ID is `V_<name>`.
pub(super) fn view(name: &str, layout: &str, filter: &str, columns: &[&str]) -> Value {
    let columns: Vec<Value> = columns
        .iter()
        .map(|column| json!({"id": column, "name": column}))
        .collect();
    json!({"id": format!("V_{name}"), "name": name, "layout": layout, "filter": filter, "fields": {"nodes": columns}})
}

/// The answer with these fields (all columns when `None`) and views.
pub(super) fn answer(fields: Option<&[&str]>, views: &[Value]) -> String {
    let all = all_columns();
    let fields: Vec<Value> = fields
        .unwrap_or(&all)
        .iter()
        .map(|name| json!({"id": name, "name": name}))
        .collect();
    json!({"data": {"node": {"fields": {"nodes": fields}, "views": {"nodes": views}}}}).to_string()
}

/// The views of a project that needs no change.
pub(super) fn complete_views(kind: ProjectKind) -> Vec<Value> {
    let views = specs(kind).iter();
    views
        .map(|spec| view(spec.name, spec.layout.graphql(), spec.filter, spec.columns))
        .collect()
}

/// The answer for a project that needs no change.
pub(in crate::setup) fn complete(kind: ProjectKind) -> String {
    answer(None, &complete_views(kind))
}

/// A fake that answers the read, then `writes` mutations.
pub(super) fn fake(read: &str, writes: usize) -> FakeGh {
    (0..writes).fold(FakeGh::new().ok("graphql", read), |gh, _| {
        gh.ok("graphql", WRITE)
    })
}

/// The input of the mutation of call `index`.
pub(super) fn input(gh: &FakeGh, index: usize) -> Value {
    crate::setup::projects::tests::sent(gh, index).1["i"].clone()
}

#[test]
fn a_complete_project_prints_only_exists() {
    let gh = fake(&complete(ProjectKind::Delivery), 0);
    let (result, lines) = run_step(&gh, &existing(ProjectKind::Delivery), false);
    result.expect("step");
    assert_eq!(lines.len(), 7);
    assert!(lines.iter().all(|line| line.starts_with("exists view ")));
    assert_eq!(gh.calls().len(), 1);
}

#[test]
fn a_project_that_does_not_exist_yet_lists_its_views() {
    let gh = FakeGh::new();
    let list = [(wanted(ProjectKind::Portfolio), None)];
    let (result, lines) = run_step(&gh, &list, true);
    result.expect("step");
    assert_eq!(lines, ["create view Initiatives", "create view Roadmap"]);
    assert_eq!(gh.calls().len(), 0);
}
