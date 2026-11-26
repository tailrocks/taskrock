//! Tests for the problems `ready` reports.

use super::tests::{has, item, listed};
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
