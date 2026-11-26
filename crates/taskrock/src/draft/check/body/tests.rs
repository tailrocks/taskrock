use super::super::tests::{BODY, epic, issue, lines};
use crate::draft::DraftIssue;

const OWNER_BODY: &str = "## Outcome\n\nO.\n\n## Question\n\nQ.\n\n## Why it matters\n\nW.\n\n## Recommended answer\n\nA.\n\n## Done when\n\n- [ ] `a` prints ok.\n";

const BUG_BODY: &str = "## What happens\n\nW.\n\n## Expected\n\nE.\n\n## How to reproduce\n\nH.\n\n## Environment\n\nV.\n";
const FEATURE_BODY: &str =
    "## Problem\n\nP.\n\n## Proposed behavior\n\nB.\n\n## Workaround today\n\nNone.\n";

fn reported(key: &str, kind: &str, extra: &str) -> DraftIssue {
    let mut full = issue(key, extra);
    full.kind = kind.to_owned();
    full.body = if kind == "Bug" {
        BUG_BODY
    } else {
        FEATURE_BODY
    }
    .to_owned();
    full
}

fn feature(key: &str, extra: &str) -> DraftIssue {
    reported(key, "Feature", extra)
}

fn full_epic() -> DraftIssue {
    let mut full = issue("E", "");
    full.kind = "Epic".to_owned();
    full
}

#[test]
fn routing_fields_are_reported() {
    let mut owner = issue("O", "parent = \"T\"");
    owner.routing.set("Agent", "Owner");
    owner.body = OWNER_BODY.to_owned();
    let mut noagent = issue("N", "");
    noagent.routing.set("Agent", "");
    let mut missing = issue("R", "");
    missing.routing.set("Review effort", "");
    let found = lines(vec![issue("T", ""), owner, noagent, missing, full_epic()]);
    assert_eq!(
        found,
        [
            "E.md: E has no children, so its type must be Task, Bug or Feature",
            "E.md: only a Task or Bug has routing fields",
            "N.md: agent is missing",
            "N.md: agent is missing",
            "O.md: an Owner task has no other routing field",
            "R.md: review_effort is missing",
            "T.md: T has children, so its type cannot be Task",
        ]
    );
}

#[test]
fn a_bug_or_feature_has_all_eight_routing_fields_or_none() {
    let bug = reported("B", "Bug", "");
    let mut partial = feature("P", "");
    partial.routing.set("Review effort", "");
    let mut bare = feature("N", "");
    bare.routing = crate::fields::Routing::default();
    let found = lines(vec![bug, partial, bare]);
    assert_eq!(
        found,
        ["P.md: a Bug or Feature has all eight routing fields or none"]
    );
}

#[test]
fn a_bug_or_feature_cannot_be_an_owner_task() {
    let mut one = feature("F", "");
    one.routing.set("Agent", "Owner");
    let mut two = reported("B", "Bug", "");
    two.routing.set("Agent", "Owner");
    let found = lines(vec![one, two]);
    assert_eq!(
        found,
        [
            "B.md: a Bug or Feature cannot be an Owner task",
            "F.md: a Bug or Feature cannot be an Owner task",
        ]
    );
}

#[test]
fn a_feature_with_children_has_no_routing_fields() {
    let found = lines(vec![feature("F", ""), issue("T", "parent = \"F\"")]);
    assert_eq!(found, ["F.md: only a Task or Bug has routing fields"]);
}

#[test]
fn sections_come_from_the_format_of_the_type() {
    let mut leaf = issue("T", "");
    leaf.body = BODY.replace("## Steps\n\nS.\n\n", "");
    let mut other = epic("E");
    other.body = BODY.replace("## Outcome\n\nO.\n\n", "## Outcome\n\n\n");
    let found = lines(vec![leaf, other, issue("C", "parent = \"E\"")]);
    assert_eq!(
        found,
        [
            "E.md: the section ## Outcome is empty. Write None. and the reason",
            "T.md: the section ## Steps is missing",
        ]
    );
}

#[test]
fn an_owner_task_uses_the_owner_format() {
    let mut owner = issue("O", "");
    owner.routing = crate::fields::Routing::default();
    owner.routing.set("Agent", "Owner");
    let found = lines(vec![owner]);
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(
        found
            .iter()
            .all(|line| line.contains("O.md: the section ## "))
    );
}

#[test]
fn done_when_boxes_are_checked() {
    let mut task = issue("T", "");
    task.body = BODY.replace("- [ ] `d` prints ok.", "- [ ] Tests pass.\n- [ ] `d`");
    let mut plain = issue("P", "");
    plain.body = BODY.replace("- [ ] `d` prints ok.", "Works.");
    assert_eq!(
        lines(vec![task, plain]),
        [
            "P.md: ## Done when has no checkbox",
            "T.md: ## Done when: box 1 has no command in backticks",
            "T.md: ## Done when: box 2 has no result after the command",
        ]
    );
}
