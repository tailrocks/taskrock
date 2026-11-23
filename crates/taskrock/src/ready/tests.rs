//! Tests for the ready rule.

use std::collections::BTreeSet;

use super::{Report, claimed, report, scope};
use crate::fields::{ROUTING, Routing};
use crate::gh::FakeGh;
use crate::github::items::Item;
use crate::link::{IssueLink, Link};

fn link(number: u64) -> IssueLink {
    IssueLink {
        owner: "tailrocks".to_owned(),
        repo: "taskrock".to_owned(),
        number,
    }
}

fn item(number: u64) -> Item {
    let mut routing = Routing::default();
    for field in ROUTING {
        routing.set(field, "x");
    }
    Item {
        item_id: number,
        link: link(number),
        issue_id: number,
        title: format!("T{number}"),
        open: true,
        kind: Some("Task".to_owned()),
        children: 0,
        open_blockers: 0,
        blocking: 0,
        status: Some("Todo".to_owned()),
        parent: None,
        routing,
    }
}

fn listed(item: &Item) -> Report<'_> {
    report(&[item], &BTreeSet::new())
}

fn numbers(items: &[&Item]) -> Vec<u64> {
    items.iter().map(|item| item.link.number).collect()
}

fn has(report: &Report<'_>, needle: &str) -> bool {
    report.problems.iter().any(|line| line.contains(needle))
}

#[test]
fn todo_without_blockers_or_claim_is_ready() {
    let task = item(1);
    let found = listed(&task);
    assert_eq!(numbers(&found.ready), [1]);
    assert_eq!((found.in_progress, found.blocked), (0, 0));
}

#[test]
fn status_or_claim_marks_in_progress() {
    let mut busy = item(1);
    busy.status = Some("In Progress".to_owned());
    assert_eq!(listed(&busy).in_progress, 1);
    let task = item(2);
    let found = report(&[&task], &BTreeSet::from([link(2)]));
    assert_eq!((found.ready.len(), found.in_progress), (0, 1));
}

#[test]
fn blocker_moves_a_todo_task_to_blocked() {
    let mut stuck = item(1);
    stuck.open_blockers = 2;
    let found = listed(&stuck);
    assert_eq!((found.ready.len(), found.blocked), (0, 1));
}

#[test]
fn owner_task_waits_on_owner() {
    let mut owned = item(1);
    owned.routing.set("Agent", "Owner");
    let found = listed(&owned);
    assert_eq!(found.waiting_on_owner.len(), 1);
    assert_eq!(found.ready.len(), 0);
}

#[test]
fn task_with_children_is_a_problem() {
    let mut parent = item(1);
    parent.children = 2;
    assert!(has(&listed(&parent), "has children"));
}

#[test]
fn candidate_without_status_is_a_problem() {
    let mut lost = item(1);
    lost.status = None;
    assert!(has(&listed(&lost), "has no status"));
}

#[test]
fn missing_routing_field_is_a_problem() {
    let mut bare = item(1);
    bare.routing = Routing::default();
    assert_eq!(listed(&bare).problems.len(), ROUTING.len());
}

#[test]
fn empty_lists_report_a_cycle() {
    let mut stuck = item(1);
    stuck.open_blockers = 1;
    assert!(has(&listed(&stuck), "blocker cycle"));
}

#[test]
fn ready_sorts_by_blocking_then_number() {
    let (mut low, mut tied, mut top) = (item(3), item(1), item(2));
    (low.blocking, tied.blocking, top.blocking) = (5, 5, 9);
    let scope = [&low, &tied, &top];
    let found = report(&scope, &BTreeSet::new());
    assert_eq!(numbers(&found.ready), [2, 1, 3]);
}

#[test]
fn scope_keeps_an_epic_and_below() {
    let (mut epic, mut task) = (item(2), item(3));
    epic.kind = Some("Epic".to_owned());
    (epic.parent, task.parent) = (Some(link(1)), Some(link(2)));
    let items = [item(1), epic, task];
    assert_eq!(numbers(&scope(&items, &Link::Issue(link(2)))), [2, 3]);
}

#[test]
fn parked_branch_is_not_a_claim() {
    let parked = r#"[[{"ref":"refs/heads/feat/issue-12-parked-1795358243"}]]"#;
    let gh = FakeGh::new().ok("matching-refs", parked);
    let task = item(12);
    assert!(claimed(&gh, &[&task]).expect("claims").is_empty());
}
