//! Tests of the stacked tasks of `ready`.

use std::collections::{BTreeMap, BTreeSet};

use super::output::{json_of, text_of};
use super::report;
use crate::config::default;
use crate::fields::Routing;
use crate::github::items::Item;
use crate::link::IssueLink;
use crate::stack::Stack;

fn link(number: u64) -> IssueLink {
    IssueLink {
        owner: "tailrocks".to_owned(),
        repo: "taskrock".to_owned(),
        number,
    }
}

fn blocked_task(number: u64) -> Item {
    Item {
        item_id: number,
        link: link(number),
        issue_id: number,
        title: format!("T{number}"),
        open: true,
        kind: Some("Task".to_owned()),
        children: 0,
        open_blockers: 1,
        blocking: 0,
        status: Some("Todo".to_owned()),
        parent: None,
        routing: Routing::default(),
    }
}

fn stack_on(blocker: u64) -> Stack {
    Stack {
        blocker: link(blocker),
        base: format!("feat/issue-{blocker}"),
        pull_request: format!("https://github.com/tailrocks/taskrock/pull/{blocker}0"),
    }
}

#[test]
fn a_stacked_task_is_ready_with_its_base() {
    let task = blocked_task(2);
    let stacks = BTreeMap::from([(link(2), stack_on(5))]);
    let found = report(
        &[&task],
        &BTreeSet::new(),
        &BTreeMap::new(),
        &default(),
        &stacks,
    );
    assert_eq!(found.ready.len(), 1);
    assert_eq!(found.blocked, 0);
    assert_eq!(found.stacked.get(&link(2)), Some(&stack_on(5)));
}

#[test]
fn a_blocked_task_is_not_ready_without_a_stack() {
    let task = blocked_task(2);
    let found = report(
        &[&task],
        &BTreeSet::new(),
        &BTreeMap::new(),
        &default(),
        &BTreeMap::new(),
    );
    assert_eq!(found.ready.len(), 0);
    assert_eq!(found.blocked, 1);
    assert_eq!(found.stacked.len(), 0);
}

#[test]
fn json_names_the_base() {
    let task = blocked_task(2);
    let stacks = BTreeMap::from([(link(2), stack_on(5))]);
    let found = report(
        &[&task],
        &BTreeSet::new(),
        &BTreeMap::new(),
        &default(),
        &stacks,
    );
    let value: serde_json::Value =
        serde_json::from_str(&json_of(&found).expect("json")).expect("parse");
    let stacked = &value["ready"][0]["stacked"];
    assert_eq!(stacked["base"], "feat/issue-5");
    assert_eq!(stacked["blocker"], 5);
    assert_eq!(
        stacked["pull_request"],
        "https://github.com/tailrocks/taskrock/pull/50"
    );
}

#[test]
fn text_names_the_base() {
    let task = blocked_task(2);
    let stacks = BTreeMap::from([(link(2), stack_on(5))]);
    let found = report(
        &[&task],
        &BTreeSet::new(),
        &BTreeMap::new(),
        &default(),
        &stacks,
    );
    let text = text_of(&found);
    assert!(
        text.contains("  stacked on: feat/issue-5 https://github.com/tailrocks/taskrock/pull/50\n"),
        "{text}"
    );
}
