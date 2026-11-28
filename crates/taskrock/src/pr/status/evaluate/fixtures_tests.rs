//! `evaluate` on the recorded pull requests of tailrocks/velnor-new.

use super::tests::codes;
use super::{State, Verdict, evaluate};
use crate::config;
use crate::pr::snapshot::case_tests::Case;
use crate::pr::snapshot::read;

fn recorded(number: u64) -> Verdict {
    let case = Case::recorded(number);
    let snapshot = read(&case.gh(), &case.link()).expect("the pull is read");
    evaluate(&snapshot, &config::default(), &case.link())
}

#[test]
fn evaluate_the_merged_pull_2932_is_merged() {
    let verdict = recorded(2932);
    assert_eq!(verdict.state, State::Merged);
    assert_eq!(
        verdict.next,
        "merged as e9830e2dc6ab82db1e47e4ca909b4b53672ce2c2"
    );
}

#[test]
fn evaluate_the_conflicting_pull_2963_is_blocked_by_the_builder() {
    let verdict = recorded(2963);
    assert_eq!(verdict.state, State::Blocked);
    assert_eq!(
        codes(&verdict),
        ["conflict", "check_failed", "closes_missing"]
    );
}

#[test]
fn evaluate_the_stacked_pull_3033_is_blocked_with_twelve_blocking_threads() {
    let verdict = recorded(3033);
    assert_eq!(verdict.state, State::Blocked);
    assert_eq!(
        codes(&verdict),
        [
            "base_not_default",
            "threads_blocking",
            "verdict_missing",
            "bot_review_pending",
            "checks_pending"
        ]
    );
    assert_eq!(verdict.findings[1].items.len(), 12);
}

#[test]
fn evaluate_the_queued_pull_3084_waits_for_the_queue() {
    let verdict = recorded(3084);
    assert_eq!(verdict.state, State::Waiting);
    assert_eq!(codes(&verdict), ["queued"]);
    assert_eq!(verdict.next, "wait for the merge queue");
}
