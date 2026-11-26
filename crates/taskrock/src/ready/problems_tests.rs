//! Tests for the problems `ready` reports.

use std::collections::{BTreeMap, BTreeSet};

use super::report;
use super::tests::{has, item, listed};
use crate::config::default;
use crate::fields::Routing;

#[test]
fn triage_item_has_no_routing_or_status_problem() {
    let mut reported = item(1);
    reported.status = Some("Triage".to_owned());
    reported.routing = Routing::default();
    assert_eq!(listed(&reported).problems, Vec::<String>::new());
}

#[test]
fn triage_item_does_not_trigger_the_cycle_hint() {
    let mut reported = item(1);
    reported.status = Some("Triage".to_owned());
    assert!(!has(&listed(&reported), "blocker cycle"));
}

#[test]
fn feature_with_children_is_a_parent_not_a_problem() {
    let mut parent = item(1);
    parent.kind = Some("Feature".to_owned());
    parent.children = 2;
    assert!(!has(&listed(&parent), "has children"));
}

#[test]
fn task_or_bug_with_children_is_still_a_problem() {
    for kind in ["Task", "Bug"] {
        let mut parent = item(1);
        parent.kind = Some(kind.to_owned());
        parent.children = 2;
        assert!(has(&listed(&parent), "has children"), "{kind}");
    }
}

#[test]
fn broken_body_alone_does_not_blame_a_blocker_cycle() {
    let task = item(1);
    let bodies = BTreeMap::from([(task.link.clone(), "No sections here.".to_owned())]);
    let found = report(&[&task], &BTreeSet::new(), &bodies, &default());
    assert!(has(&found, "breaks the Task format"));
    assert!(!has(&found, "blocker cycle"));
}

#[test]
fn broken_body_task_does_not_trigger_the_cycle_hint() {
    let broken = item(1);
    let mut blocked = item(2);
    blocked.open_blockers = 1;
    let bodies = BTreeMap::from([(broken.link.clone(), "No sections here.".to_owned())]);
    let found = report(&[&broken, &blocked], &BTreeSet::new(), &bodies, &default());
    assert!(has(&found, "breaks the"));
    assert!(!has(&found, "blocker cycle"));
}

#[test]
fn blocked_task_with_no_ready_task_still_triggers_the_cycle_hint() {
    let (mut first, mut second) = (item(1), item(2));
    first.open_blockers = 1;
    second.open_blockers = 1;
    let found = report(
        &[&first, &second],
        &BTreeSet::new(),
        &BTreeMap::new(),
        &default(),
    );
    assert!(has(&found, "blocker cycle"));
}
