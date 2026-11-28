//! One test per wait code, and the Codex rules.

use super::{HEAD, State, check, codes, evaluate, judge, link, ready};
use crate::config::{self, Config};
use crate::pr::snapshot::{CodexFact, QueueEntry};

/// The default config, with the Codex connector named for the repository of `link()`.
fn codex_repo(name: &str) -> Config {
    let mut config = config::default();
    config.review.codex = vec![name.into()];
    config
}

fn codex(code_sha: &str, code: &str, security_sha: &str, security: &str) -> CodexFact {
    CodexFact {
        code_sha: code_sha.into(),
        code_status: code.into(),
        security_sha: security_sha.into(),
        security_status: security.into(),
    }
}

#[test]
fn evaluate_mergeability_pending_when_github_has_not_decided() {
    let mut snapshot = ready();
    snapshot.mergeable = None;
    assert_eq!(codes(&judge(&snapshot)), ["mergeability_pending"]);
}

#[test]
fn evaluate_checks_pending_for_a_required_name_with_no_fact() {
    let mut snapshot = ready();
    snapshot.checks.clear();
    let verdict = judge(&snapshot);
    assert_eq!(codes(&verdict), ["checks_pending"]);
    assert_eq!(verdict.findings[0].items, ["Required"]);
}

#[test]
fn evaluate_checks_pending_for_a_check_that_is_not_completed() {
    let mut snapshot = ready();
    snapshot.checks = vec![crate::pr::snapshot::CheckFact {
        name: "Required".into(),
        completed: false,
        conclusion: None,
    }];
    assert_eq!(codes(&judge(&snapshot)), ["checks_pending"]);
}

#[test]
fn evaluate_a_check_that_is_not_required_is_not_judged() {
    let mut snapshot = ready();
    snapshot.checks.push(check("Extra", Some("failure")));
    assert_eq!(judge(&snapshot).findings, []);
}

#[test]
fn evaluate_bot_review_pending_while_codex_runs() {
    let mut snapshot = ready();
    snapshot.codex = Some(codex(HEAD, "completed", HEAD, "running"));
    let verdict = judge(&snapshot);
    assert_eq!(codes(&verdict), ["bot_review_pending"]);
    assert_eq!(
        verdict.next,
        "wait for the Codex code and security review of 1234567"
    );
}

#[test]
fn evaluate_bot_review_pending_when_codex_reviewed_another_head() {
    let mut snapshot = ready();
    let old = "f".repeat(40);
    snapshot.codex = Some(codex(HEAD, "completed", &old, "completed"));
    assert_eq!(codes(&judge(&snapshot)), ["bot_review_pending"]);
    snapshot.codex = Some(codex(&old, "completed", HEAD, "completed"));
    assert_eq!(codes(&judge(&snapshot)), ["bot_review_pending"]);
}

#[test]
fn evaluate_a_completed_codex_review_of_the_head_gives_nothing() {
    let mut snapshot = ready();
    snapshot.codex = Some(codex(HEAD, "completed", HEAD, "completed"));
    assert_eq!(judge(&snapshot).findings, []);
}

#[test]
fn evaluate_no_codex_comment_in_a_listed_repository_is_bot_review_pending() {
    let snapshot = ready();
    let verdict = evaluate(&snapshot, &codex_repo("r"), &link());
    assert_eq!(codes(&verdict), ["bot_review_pending"]);
    assert_eq!(
        verdict.next,
        "wait for the Codex code and security review of 1234567"
    );
    let verdict = evaluate(&snapshot, &codex_repo("R"), &link());
    assert_eq!(codes(&verdict), ["bot_review_pending"]);
}

#[test]
fn evaluate_no_codex_comment_in_an_unlisted_repository_gives_nothing() {
    let snapshot = ready();
    assert_eq!(judge(&snapshot).findings, []);
    let verdict = evaluate(&snapshot, &codex_repo("other"), &link());
    assert_eq!(verdict.findings, []);
}

#[test]
fn evaluate_no_codex_comment_on_a_draft_gives_only_the_draft() {
    let mut snapshot = ready();
    snapshot.draft = true;
    let verdict = evaluate(&snapshot, &codex_repo("r"), &link());
    assert_eq!(codes(&verdict), ["draft"]);
}

#[test]
fn evaluate_a_completed_codex_review_in_a_listed_repository_gives_nothing() {
    let mut snapshot = ready();
    snapshot.codex = Some(codex(HEAD, "completed", HEAD, "completed"));
    assert_eq!(evaluate(&snapshot, &codex_repo("r"), &link()).findings, []);
}

#[test]
fn evaluate_queued_when_the_pull_request_is_in_the_queue() {
    let mut snapshot = ready();
    snapshot.queue.entry = Some(QueueEntry {
        state: "AWAITING_CHECKS".into(),
        position: Some(1),
    });
    assert_eq!(codes(&judge(&snapshot)), ["queued"]);
}

#[test]
fn evaluate_bot_review_pending_while_the_code_review_runs() {
    let mut snapshot = ready();
    snapshot.codex = Some(codex(HEAD, "running", HEAD, "completed"));
    assert_eq!(codes(&judge(&snapshot)), ["bot_review_pending"]);
}

#[test]
fn evaluate_checks_pending_when_no_required_check_is_readable_and_none_has_started() {
    let mut snapshot = ready();
    snapshot.required_checks.clear();
    snapshot.checks.clear();
    let verdict = judge(&snapshot);
    assert_eq!(codes(&verdict), ["checks_pending"]);
    assert_eq!(
        verdict.findings[0].detail,
        "No check has started on the head yet."
    );
    assert_eq!(verdict.state, State::Waiting);
}

#[test]
fn evaluate_one_completed_check_is_enough_when_no_required_check_is_readable() {
    let mut snapshot = ready();
    snapshot.required_checks.clear();
    snapshot.checks = vec![check("Any", Some("success"))];
    assert_eq!(judge(&snapshot).findings, []);
}

#[test]
fn evaluate_readable_required_checks_keep_their_rule_when_other_checks_exist() {
    let mut snapshot = ready();
    snapshot.checks = vec![check("Extra", Some("success"))];
    let verdict = judge(&snapshot);
    assert_eq!(codes(&verdict), ["checks_pending"]);
    assert_eq!(verdict.findings[0].items, ["Required"]);
}
