//! The rules about the pull request itself and its checks.

use super::Finding;
use crate::pr::snapshot::{CheckFact, PullSnapshot};
use crate::pr::status::code::Code;
use crate::stack;

/// A conclusion that does not stop the merge.
const PASSING: [&str; 3] = ["success", "neutral", "skipped"];

pub(super) fn problems(snapshot: &PullSnapshot, out: &mut Vec<Finding>) {
    let no_conclusion = snapshot
        .checks
        .iter()
        .filter(|check| check.completed && check.conclusion.is_none())
        .map(|check| format!("The check {} completed with no conclusion.", check.name));
    let items: Vec<String> = snapshot
        .problems
        .iter()
        .cloned()
        .chain(no_conclusion)
        .collect();
    if !items.is_empty() {
        out.push(
            Finding::new(
                Code::UnknownState,
                "taskrock could not read every fact of the pull request.",
                "run taskrock pr status again; when it repeats, file a taskrock Bug",
            )
            .items(items),
        );
    }
}

pub(super) fn base(snapshot: &PullSnapshot, out: &mut Vec<Finding>) {
    if snapshot.base_ref != snapshot.default_branch {
        out.push(Finding::new(
            Code::BaseNotDefault,
            format!(
                "The base is {}, not the default branch {}.",
                snapshot.base_ref, snapshot.default_branch
            ),
            "taskrock pr merge of the base pull request retargets this one",
        ));
    }
}

/// A conflict, or the wait for GitHub to compute whether there is one.
pub(super) fn mergeability(snapshot: &PullSnapshot, out: &mut Vec<Finding>) {
    if snapshot.mergeable == Some(false) || snapshot.mergeable_state == "dirty" {
        out.push(Finding::new(
            Code::Conflict,
            format!("The branch conflicts with {}.", snapshot.base_ref),
            format!(
                "merge origin/{} into the branch, resolve the conflicts and push",
                snapshot.base_ref
            ),
        ));
    } else if snapshot.mergeable.is_none() {
        out.push(Finding::new(
            Code::MergeabilityPending,
            "GitHub has not computed whether the branch merges.",
            "wait for GitHub, then run taskrock pr status again",
        ));
    }
}

/// The required checks, or every check when no required check can be read. With none
/// readable and no check on the head, CI has not started: a wait.
pub(super) fn checks(snapshot: &PullSnapshot, out: &mut Vec<Finding>) {
    if snapshot.required_checks.is_empty() && snapshot.checks.is_empty() {
        out.push(Finding::new(
            Code::ChecksPending,
            "No check has started on the head yet.",
            "wait for the checks to start",
        ));
        return;
    }
    let listed: Vec<&str> = if snapshot.required_checks.is_empty() {
        snapshot
            .checks
            .iter()
            .map(|check| check.name.as_str())
            .collect()
    } else {
        snapshot
            .required_checks
            .iter()
            .map(String::as_str)
            .collect()
    };
    let mut names: Vec<&str> = Vec::new();
    for name in listed {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    let (mut pending, mut failed) = (Vec::new(), Vec::new());
    for name in names {
        let facts: Vec<&CheckFact> = snapshot.checks.iter().filter(|c| c.name == name).collect();
        if facts.is_empty() || facts.iter().any(|check| !check.completed) {
            pending.push(name.to_owned());
        } else if facts.iter().any(|check| fails(check)) {
            failed.push(name.to_owned());
        }
    }
    if !failed.is_empty() {
        out.push(
            Finding::new(
                Code::CheckFailed,
                format!("{} check(s) failed.", failed.len()),
                "fix the failed checks and push",
            )
            .items(failed),
        );
    }
    if !pending.is_empty() {
        out.push(
            Finding::new(
                Code::ChecksPending,
                format!("{} check(s) have not finished.", pending.len()),
                "wait for the checks to finish",
            )
            .items(pending),
        );
    }
}

/// A completed check whose conclusion stops the merge. A missing conclusion is `unknown_state`.
fn fails(check: &CheckFact) -> bool {
    check
        .conclusion
        .as_deref()
        .is_some_and(|conclusion| !PASSING.contains(&conclusion))
}

pub(super) fn draft(snapshot: &PullSnapshot, out: &mut Vec<Finding>) {
    if snapshot.draft {
        out.push(Finding::new(
            Code::Draft,
            "The pull request is a draft.",
            "mark the pull request ready for review",
        ));
    }
}

/// A `feat/issue-<n>` branch needs the line `Closes #<n>` in the body.
pub(super) fn closes(snapshot: &PullSnapshot, out: &mut Vec<Finding>) {
    let Some(number) = stack::issue_number(&snapshot.head_ref) else {
        return;
    };
    let line = format!("closes #{number}");
    if !snapshot
        .body
        .lines()
        .any(|l| l.trim().eq_ignore_ascii_case(&line))
    {
        out.push(Finding::new(
            Code::ClosesMissing,
            format!("The body has no line `Closes #{number}`."),
            format!("add the line `Closes #{number}` to the pull request body"),
        ));
    }
}

pub(super) fn queue(snapshot: &PullSnapshot, out: &mut Vec<Finding>) {
    if snapshot.queue.entry.is_some() {
        out.push(Finding::new(
            Code::Queued,
            "The pull request is in the merge queue.",
            "wait for the merge queue",
        ));
    }
}
