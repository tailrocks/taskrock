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
