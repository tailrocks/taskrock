//! Helpers for the tests of the steps that take the list of projects.

use crate::config::{Project, ProjectKind, SharedMilestone};
use crate::error::Error;
use crate::gh::FakeGh;
use crate::projects::OrgProject;
use crate::setup::Run;
use crate::setup::project_fields::tests::{found, wanted};

/// The delivery project `taskrock` with these repositories and milestones.
/// It exists on GitHub unless `exists` is false, as in a dry run.
pub(in crate::setup) fn project(
    repos: &[&str],
    milestones: Vec<SharedMilestone>,
    exists: bool,
) -> (Project, Option<OrgProject>) {
    let wanted = Project {
        repos: repos.iter().map(|name| (*name).to_owned()).collect(),
        milestones,
        ..wanted(ProjectKind::Delivery)
    };
    (wanted, exists.then(found))
}

/// Run one step with `acme` as the owner. Returns its result and its lines.
pub(in crate::setup) fn step(
    gh: &FakeGh,
    dry_run: bool,
    step: impl FnOnce(&mut Run<'_>) -> Result<(), Error>,
) -> (Result<(), Error>, Vec<String>) {
    let mut run = Run {
        gh,
        owner: "acme".to_owned(),
        dry_run,
        yes: false,
        lines: Vec::new(),
    };
    let result = step(&mut run);
    (result, run.lines)
}

/// The JSON input of the REST call `index`.
pub(in crate::setup) fn body(gh: &FakeGh, index: usize) -> serde_json::Value {
    let input = gh.calls()[index].1.clone().unwrap_or_default();
    serde_json::from_str(&input).unwrap_or_default()
}
