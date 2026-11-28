//! The fake answers and the runner of the workflows tests.

use serde_json::{Value, json};

use super::super::{NAMES, workflows};
use crate::config::Workflows;
use crate::error::Error;
use crate::gh::FakeGh;
use crate::setup::tests::testing::{project, step};

/// The answer with these workflows (name, enabled) and these views (name,
/// layout, group-by field names).
pub(super) fn answer(present: &[(&str, bool)], views: &[(&str, &str, &[&str])]) -> String {
    let workflows: Vec<Value> = present
        .iter()
        .map(|(name, enabled)| json!({"id": format!("W_{name}"), "name": name, "enabled": enabled}))
        .collect();
    let views: Vec<Value> = views
        .iter()
        .map(|(name, layout, groups)| {
            let fields: Vec<Value> = groups.iter().map(|name| json!({"name": name})).collect();
            json!({"name": name, "layout": layout, "groupByFields": {"nodes": fields}})
        })
        .collect();
    json!({"data": {"node": {"workflows": {"nodes": workflows}, "views": {"nodes": views}}}})
        .to_string()
}

pub(super) fn all_on() -> Vec<(&'static str, bool)> {
    NAMES.iter().map(|name| (*name, true)).collect()
}

/// Run the step on the project `taskrock` with these workflows. It exists on
/// GitHub unless `exists` is false. Returns the result and the lines.
pub(super) fn run_step(
    gh: &FakeGh,
    set: Workflows,
    exists: bool,
    yes: bool,
    dry_run: bool,
) -> (Result<(), Error>, Vec<String>) {
    let (mut wanted, found) = project(&[], Vec::new(), exists);
    wanted.workflows = set;
    step(gh, dry_run, |run| {
        run.yes = yes;
        workflows(run, &[(wanted, found)])
    })
}

pub(super) fn off(set: fn(&mut Workflows)) -> Workflows {
    let mut workflows = Workflows::default();
    set(&mut workflows);
    workflows
}

pub(super) fn only_exists(lines: &[String]) -> bool {
    lines.len() == 6
        && lines
            .iter()
            .all(|line| line.starts_with("exists workflow "))
}
