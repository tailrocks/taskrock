//! Tests of the Triage group of `ready`.

use std::collections::{BTreeMap, BTreeSet};

use super::{is_candidate, report, text_of};
use crate::config::default;
use crate::fields::Routing;
use crate::github::items::Item;
use crate::link::IssueLink;

fn triaged(number: u64, kind: &str) -> Item {
    Item {
        item_id: number,
        link: IssueLink {
            owner: "tailrocks".to_owned(),
            repo: "taskrock".to_owned(),
            number,
        },
        issue_id: number,
        title: format!("T{number}"),
        open: true,
        kind: Some(kind.to_owned()),
        children: 0,
        open_blockers: 0,
        blocking: 0,
        status: Some("Triage".to_owned()),
        parent: None,
        milestone: None,
        routing: Routing::default(),
    }
}

#[test]
fn triage_item_is_listed_and_is_neither_candidate_nor_problem() {
    let bug = triaged(7, "Bug");
    assert!(!is_candidate(&bug));
    let found = report(
        &[&bug],
        &BTreeSet::new(),
        &BTreeMap::new(),
        &default(),
        &BTreeMap::new(),
    );
    assert_eq!(found.triage.len(), 1);
    assert_eq!(found.ready.len(), 0);
    assert!(found.problems.is_empty(), "{:?}", found.problems);
}

#[test]
fn closed_triage_item_is_not_listed() {
    let mut bug = triaged(7, "Bug");
    bug.open = false;
    let found = report(
        &[&bug],
        &BTreeSet::new(),
        &BTreeMap::new(),
        &default(),
        &BTreeMap::new(),
    );
    assert_eq!(found.triage.len(), 0);
}

#[test]
fn text_has_a_triage_section_with_the_type() {
    let (bug, feature) = (triaged(7, "Bug"), triaged(8, "Feature"));
    let found = report(
        &[&bug, &feature],
        &BTreeSet::new(),
        &BTreeMap::new(),
        &default(),
        &BTreeMap::new(),
    );
    let text = text_of(&found);
    assert!(text.contains("TRIAGE 2\n"), "{text}");
    assert!(
        text.contains("https://github.com/tailrocks/taskrock/issues/7  T7 (Bug)\n"),
        "{text}"
    );
    assert!(text.ends_with("PROBLEMS 0"), "{text}");
}
