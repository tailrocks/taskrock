//! One test per blocker code.

use super::{HEAD, check, codes, evaluate, judge, link, ready, thread, verdict};
use crate::config;

const P2: &str = "![P2 Badge](https://img.shields.io/badge/P2-yellow)";
const P3: &str = "![P3 Badge](https://img.shields.io/badge/P3-green)";

#[test]
fn evaluate_unknown_state_lists_the_read_problems() {
    let mut snapshot = ready();
    snapshot.problems = vec!["The comments could not be read: boom.".into()];
    let verdict = judge(&snapshot);
    assert_eq!(codes(&verdict), ["unknown_state"]);
    assert_eq!(verdict.findings[0].items, snapshot.problems);
}

#[test]
fn evaluate_unknown_state_for_a_completed_check_with_no_conclusion() {
    let mut snapshot = ready();
    snapshot.checks = vec![check("Required", None)];
    assert_eq!(codes(&judge(&snapshot)), ["unknown_state"]);
}

#[test]
fn evaluate_base_not_default_names_the_retarget() {
    let mut snapshot = ready();
    snapshot.base_ref = "feat/issue-4".into();
    let verdict = judge(&snapshot);
    assert_eq!(codes(&verdict), ["base_not_default"]);
    assert_eq!(
        verdict.next,
        "taskrock pr merge of the base pull request retargets this one"
    );
}

#[test]
fn evaluate_conflict_from_mergeable_false_or_a_dirty_state() {
    let mut snapshot = ready();
    snapshot.mergeable = Some(false);
    assert_eq!(codes(&judge(&snapshot)), ["conflict"]);
    snapshot.mergeable = None;
    snapshot.mergeable_state = "dirty".into();
    assert_eq!(codes(&judge(&snapshot)), ["conflict"]);
}

#[test]
fn evaluate_other_mergeable_states_are_not_judged() {
    for state in ["behind", "blocked", "unstable", "has_hooks", "unknown"] {
        let mut snapshot = ready();
        snapshot.mergeable_state = state.into();
        assert_eq!(judge(&snapshot).findings, [], "{state}");
    }
}

#[test]
fn evaluate_rounds_exhausted_at_the_limit_with_no_pass() {
    let mut config = config::default();
    config.review.max_rounds = 2;
    let mut snapshot = ready();
    snapshot.verdicts = vec![verdict(false, &"a".repeat(40), 1), verdict(false, HEAD, 2)];
    let found = evaluate(&snapshot, &config, &link());
    assert_eq!(codes(&found), ["rounds_exhausted", "verdict_failed"]);
}

#[test]
fn evaluate_a_pass_then_a_fail_on_one_head_is_not_rounds_exhausted() {
    let mut config = config::default();
    config.review.max_rounds = 1;
    let mut snapshot = ready();
    snapshot.verdicts = vec![verdict(true, HEAD, 1), verdict(false, HEAD, 2)];
    let found = evaluate(&snapshot, &config, &link());
    assert_eq!(codes(&found), ["verdict_failed"]);
}

#[test]
fn evaluate_threads_blocking_lists_the_urls_and_skips_resolved() {
    let mut snapshot = ready();
    let mut done = thread("done", true, P2);
    done.resolved = true;
    snapshot.threads = vec![
        thread("one", true, P2),
        thread("two", true, "no badge"),
        done,
    ];
    let verdict = judge(&snapshot);
    assert_eq!(codes(&verdict), ["threads_blocking"]);
    assert_eq!(verdict.findings[0].items, ["one", "two"]);
    assert!(verdict.next.starts_with("reviewer: reproduce it; reproduced goes in the FAIL, otherwise taskrock pr threads https://github.com/o/r/pull/9 --resolve <id> --reason"));
}

#[test]
fn evaluate_threads_human_for_a_person() {
    let mut snapshot = ready();
    snapshot.threads = vec![thread("p", false, P2)];
    assert_eq!(codes(&judge(&snapshot)), ["threads_human"]);
}

#[test]
fn evaluate_threads_deferrable_for_a_p3_bot() {
    let mut snapshot = ready();
    snapshot.threads = vec![thread("p", true, P3)];
    let verdict = judge(&snapshot);
    assert_eq!(codes(&verdict), ["threads_deferrable"]);
    assert_eq!(
        verdict.next,
        "taskrock pr threads https://github.com/o/r/pull/9 --defer"
    );
}

#[test]
fn evaluate_check_failed_for_a_conclusion_that_is_not_passing() {
    let mut snapshot = ready();
    snapshot.checks = vec![check("Required", Some("failure"))];
    let verdict = judge(&snapshot);
    assert_eq!(codes(&verdict), ["check_failed"]);
    assert_eq!(verdict.findings[0].items, ["Required"]);
    for ok in ["neutral", "skipped"] {
        snapshot.checks = vec![check("Required", Some(ok))];
        assert_eq!(judge(&snapshot).findings, [], "{ok}");
    }
}

#[test]
fn evaluate_with_no_required_names_every_check_counts() {
    let mut snapshot = ready();
    snapshot.required_checks.clear();
    snapshot.checks = vec![check("a", Some("success")), check("b", Some("cancelled"))];
    let verdict = judge(&snapshot);
    assert_eq!(codes(&verdict), ["check_failed"]);
    assert_eq!(verdict.findings[0].items, ["b"]);
}

#[test]
fn evaluate_verdict_failed_when_the_newest_for_the_head_is_fail() {
    let mut snapshot = ready();
    snapshot.verdicts = vec![verdict(true, HEAD, 1), verdict(false, HEAD, 2)];
    assert_eq!(codes(&judge(&snapshot)), ["verdict_failed"]);
}

#[test]
fn evaluate_verdict_missing_names_the_newest_older_head() {
    let mut snapshot = ready();
    snapshot.verdicts = vec![
        verdict(true, &"a".repeat(40), 1),
        verdict(false, &"b".repeat(40), 2),
    ];
    let verdict = judge(&snapshot);
    assert_eq!(codes(&verdict), ["verdict_missing"]);
    assert!(
        verdict.findings[0]
            .detail
            .contains("older verdict is for bbbbbbb")
    );
    snapshot.verdicts.clear();
    assert!(!judge(&snapshot).findings[0].detail.contains("older"));
}

#[test]
fn evaluate_draft_blocks() {
    let mut snapshot = ready();
    snapshot.draft = true;
    assert_eq!(codes(&judge(&snapshot)), ["draft"]);
}

#[test]
fn evaluate_closes_missing_for_a_task_branch_without_the_line() {
    let mut snapshot = ready();
    snapshot.body = "Closes #50\nPart of #5".into();
    assert_eq!(codes(&judge(&snapshot)), ["closes_missing"]);
    snapshot.body = "text\n  closes #5  \n".into();
    assert_eq!(judge(&snapshot).findings, []);
    snapshot.head_ref = "feat/issue-5-more".into();
    snapshot.body.clear();
    assert_eq!(judge(&snapshot).findings, []);
}

#[test]
fn evaluate_a_pass_for_another_head_does_not_stop_rounds_exhausted() {
    let mut config = config::default();
    config.review.max_rounds = 2;
    let mut snapshot = ready();
    snapshot.verdicts = vec![verdict(true, &"a".repeat(40), 1), verdict(false, HEAD, 2)];
    let found = evaluate(&snapshot, &config, &link());
    assert_eq!(codes(&found), ["rounds_exhausted", "verdict_failed"]);
}
