//! The rules about the text of each issue: routing, sections, checkboxes, bodies and titles.

use crate::draft::{Draft, DraftIssue};
use crate::fields::{ROUTING, draft_key};

use super::{Finding, at, is_epic, is_task};

pub(super) fn routing(draft: &Draft, out: &mut Vec<Finding>) {
    for issue in &draft.issues {
        if is_epic(&issue.kind) && has_routing(issue, 0) {
            let message = "only a Task or Bug has routing fields".to_owned();
            out.push(at(issue, message));
        }
        if !is_task(&issue.kind) {
            continue;
        }
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
}

pub(super) fn has_routing(issue: &DraftIssue, skip: usize) -> bool {
    ROUTING.iter().skip(skip).any(|field| {
        issue
            .routing
            .get(field)
            .is_some_and(|value| !value.is_empty())
    })
}

pub(super) fn sections(draft: &Draft, out: &mut Vec<Finding>) {
    for issue in &draft.issues {
        let leaf = is_task(&issue.kind) && !issue.routing.is_owner();
        let wanted: &[&str] = if leaf {
            &[
                "## Outcome",
                "## Decisions already made",
                "## Steps",
                "## Done when",
                "## Out of scope",
            ]
        } else {
            &["## Outcome", "## Done when"]
        };
        for header in wanted {
            if !has_section(&issue.body, header) {
                out.push(at(issue, format!("the section {header} is missing")));
            }
        }
    }
}

pub(super) fn has_section(body: &str, header: &str) -> bool {
    body.lines().any(|line| line.trim() == header)
}

pub(super) fn checkboxes(draft: &Draft, out: &mut Vec<Finding>) {
    for issue in &draft.issues {
        if has_section(&issue.body, "## Done when") && !has_checkbox(&issue.body) {
            out.push(at(issue, "## Done when has no checkbox".to_owned()));
        }
    }
}

pub(super) fn has_checkbox(body: &str) -> bool {
    let mut lines = body
        .lines()
        .skip_while(|line| line.trim() != "## Done when");
    lines.next();
    lines
        .take_while(|line| !line.trim().starts_with("## "))
        .any(|line| line.trim_start().starts_with("- [ ]"))
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
        }
    }
}
