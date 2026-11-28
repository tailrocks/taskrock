//! The order of the findings, the state they give and the rounds.

use super::super::State;
use super::{HEAD, codes, judge, ready, verdict};
use crate::pr::snapshot::QueueEntry;

#[test]
fn evaluate_orders_blockers_before_waits_each_in_code_order() {
    let mut snapshot = ready();
    snapshot.draft = true;
    snapshot.checks = vec![];
    snapshot.mergeable = None;
    snapshot.base_ref = "feat/issue-4".into();
    snapshot.queue.entry = Some(QueueEntry {
        state: "QUEUED".into(),
        position: Some(1),
    });
    let verdict = judge(&snapshot);
    assert_eq!(
        codes(&verdict),
        [
            "base_not_default",
            "draft",
            "mergeability_pending",
            "checks_pending",
            "queued"
        ]
    );
    assert_eq!(verdict.state, State::Blocked);
    assert_eq!(verdict.next, verdict.findings[0].next);
}

#[test]
fn evaluate_only_waits_gives_waiting_with_the_first_wait_next() {
    let mut snapshot = ready();
    snapshot.mergeable = None;
    let verdict = judge(&snapshot);
    assert_eq!(verdict.state, State::Waiting);
    assert_eq!(verdict.next, verdict.findings[0].next);
}

#[test]
fn evaluate_counts_rounds_by_distinct_head() {
    let other = "a".repeat(40);
    let mut snapshot = ready();
    snapshot.verdicts = vec![
        verdict(false, &other, 1),
        verdict(false, &other, 2),
        verdict(false, HEAD, 3),
    ];
    assert_eq!(codes(&judge(&snapshot)), ["verdict_failed"]);
    snapshot
        .verdicts
        .insert(0, verdict(false, &"b".repeat(40), 0));
    assert_eq!(
        codes(&judge(&snapshot)),
        ["rounds_exhausted", "verdict_failed"]
    );
    snapshot.verdicts.push(verdict(true, HEAD, 4));
    assert_eq!(judge(&snapshot).findings, []);
}
