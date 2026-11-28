//! Tests for the loose issues of a milestone.

use std::collections::{BTreeMap, BTreeSet};

use super::tests::item;
use super::{loose, report, text_of};
use crate::config::default;
use crate::github::items::Item;
use crate::link::{IssueLink, Link};

fn in_milestone(number: u64, kind: &str, milestone: Option<&str>) -> Item {
    let mut found = item(number);
    found.kind = Some(kind.to_owned());
    found.milestone = milestone.map(str::to_owned);
    found
}

fn issue(number: u64) -> IssueLink {
    IssueLink {
        owner: "tailrocks".to_owned(),
        repo: "taskrock".to_owned(),
        number,
    }
}

fn link_of(number: u64) -> Link {
    Link::Issue(issue(number))
}

fn numbers(items: &[&Item]) -> Vec<u64> {
    items.iter().map(|item| item.link.number).collect()
}

#[test]
fn loose_holds_open_parentless_items_of_the_milestone() {
    let items = [
        in_milestone(1, "Epic", Some("M1")),
        in_milestone(2, "Task", Some("M1")),
        in_milestone(3, "Bug", Some("M1")),
        in_milestone(4, "Feature", Some("M1")),
        in_milestone(5, "Epic", Some("M1")),
    ];
    let found = loose(&items, &link_of(1));
    assert_eq!(numbers(&found), vec![2, 3, 4, 5]);
}

#[test]
fn loose_leaves_out_what_does_not_match() {
    let mut closed = in_milestone(2, "Task", Some("M1"));
    closed.open = false;
    let mut child = in_milestone(3, "Task", Some("M1"));
    child.parent = Some(issue(1));
    let items = [
        in_milestone(1, "Epic", Some("M1")),
        closed,
        child,
        in_milestone(4, "Task", Some("M2")),
        in_milestone(5, "Task", None),
        in_milestone(6, "Initiative", Some("M1")),
    ];
    assert_eq!(numbers(&loose(&items, &link_of(1))), Vec::<u64>::new());
}

#[test]
fn loose_is_empty_when_the_link_has_no_milestone() {
    let items = [
        in_milestone(1, "Epic", None),
        in_milestone(2, "Task", None),
        in_milestone(3, "Task", Some("M1")),
    ];
    assert_eq!(numbers(&loose(&items, &link_of(1))), Vec::<u64>::new());
}

#[test]
fn loose_is_empty_for_a_project_link() {
    let items = [
        in_milestone(1, "Epic", Some("M1")),
        in_milestone(2, "Task", Some("M1")),
    ];
    let project = "https://github.com/orgs/tailrocks/projects/3".parse::<Link>();
    assert_eq!(
        numbers(&loose(&items, &project.expect("project link"))),
        Vec::<u64>::new()
    );
}

#[test]
fn text_lists_loose_after_triage_and_only_when_not_empty() {
    let items = [
        in_milestone(1, "Epic", Some("M1")),
        in_milestone(2, "Bug", Some("M1")),
    ];
    let mut found = report(
        &[],
        &BTreeSet::new(),
        &BTreeMap::new(),
        &default(),
        &BTreeMap::new(),
    );
    assert!(!text_of(&found).contains("LOOSE"));
    found.loose = loose(&items, &link_of(1));
    let text = text_of(&found);
    let want =
        "TRIAGE 0\nLOOSE 1\nhttps://github.com/tailrocks/taskrock/issues/2  T2 (Bug)\nPROBLEMS 0";
    assert!(text.contains(want), "{text}");
}
