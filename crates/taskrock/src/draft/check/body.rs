//! The rules about the text of each issue: routing, sections, bare numbers, bodies and titles.

use crate::config::Config;
use crate::draft::{Draft, DraftIssue};
use crate::fields::{ROUTING, draft_key};
use crate::format;

use super::{Finding, at, has_children};

#[cfg(test)]
mod tests;

pub(super) fn routing(draft: &Draft, out: &mut Vec<Finding>) {
    let children = has_children(draft);
    for issue in &draft.issues {
        let parent = children.contains_key(issue.key.as_str());
        match issue.kind.as_str() {
            "Initiative" | "Epic" => no_routing(issue, out),
            "Feature" if parent => no_routing(issue, out),
            "Task" => task_routing(issue, out),
            "Bug" | "Feature" => report_routing(issue, out),
            _ => {}
        }
    }
}

fn no_routing(issue: &DraftIssue, out: &mut Vec<Finding>) {
    if has_routing(issue, 0) {
        let message = "only a Task or Bug has routing fields".to_owned();
        out.push(at(issue, message));
    }
}

fn task_routing(issue: &DraftIssue, out: &mut Vec<Finding>) {
    if issue.routing.get("Agent").is_none_or(str::is_empty) {
        out.push(at(issue, "agent is missing".to_owned()));
    }
    if issue.routing.is_owner() && has_routing(issue, 1) {
        let message = "an Owner task has no other routing field".to_owned();
        out.push(at(issue, message));
    }
    if !issue.routing.is_owner() {
        for field in issue.routing.missing() {
            out.push(at(issue, format!("{} is missing", draft_key(field))));
        }
    }
}

fn report_routing(issue: &DraftIssue, out: &mut Vec<Finding>) {
    if issue.routing.is_owner() {
        let message = "a Bug or Feature cannot be an Owner task".to_owned();
        out.push(at(issue, message));
    } else if has_routing(issue, 0) && !issue.routing.missing().is_empty() {
        let message = "a Bug or Feature has all eight routing fields or none".to_owned();
        out.push(at(issue, message));
    }
}

pub(super) fn has_routing(issue: &DraftIssue, skip: usize) -> bool {
    ROUTING.iter().skip(skip).any(|field| {
        issue
            .routing
            .get(field)
            .is_some_and(|value| !value.is_empty())
    })
}

pub(super) fn sections(draft: &Draft, config: &Config, out: &mut Vec<Finding>) {
    for issue in &draft.issues {
        let owner = issue.kind == "Task" && issue.routing.is_owner();
        let Some(format) = config.format(&issue.kind, owner) else {
            continue;
        };
        for message in format::problems(&issue.body, &format.file, &format.checklist) {
            out.push(at(issue, message));
        }
    }
}

pub(super) fn bare_numbers(draft: &Draft, out: &mut Vec<Finding>) {
    for issue in &draft.issues {
        for message in format::bare_numbers(&issue.body) {
            out.push(at(issue, message));
        }
    }
}

pub(super) fn bodies(draft: &Draft, out: &mut Vec<Finding>) {
    for issue in &draft.issues {
        let count = issue.body.chars().count();
        if count > 60_000 {
            let message = format!("the body has {count} characters. The limit is 60000");
            out.push(at(issue, message));
        }
    }
}

pub(super) fn titles(draft: &Draft, out: &mut Vec<Finding>) {
    for issue in &draft.issues {
        let count = issue.title.chars().count();
        if count == 0 || count > 256 {
            let message = "the title must have 1 to 256 characters".to_owned();
            out.push(at(issue, message));
        } else if starts_lowercase(issue) {
            let message = "the title starts with a lowercase letter. Start with a capital letter or a digit, put the code name in backticks, or start with the name of the repository".to_owned();
            out.push(at(issue, message));
        }
    }
}

/// A lowercase first letter is allowed only when the title starts with the repository
/// name of the issue, followed by a space or a colon.
fn starts_lowercase(issue: &DraftIssue) -> bool {
    if !issue.title.starts_with(|c: char| c.is_ascii_lowercase()) {
        return false;
    }
    let name = issue.repo.rsplit('/').next().unwrap_or_default();
    let named = issue
        .title
        .strip_prefix(name)
        .is_some_and(|rest| rest.starts_with([' ', ':']));
    !named
}
