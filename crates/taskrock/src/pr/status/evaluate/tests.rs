//! Tests of `evaluate`: the states, the order and the rounds.

use super::{State, Verdict, evaluate};
use crate::config;
use crate::link::PullLink;
use crate::pr::snapshot::{CheckFact, PullSnapshot, PullState, QueueFact, ThreadFact, VerdictFact};

mod blockers;
mod fixtures;
mod order;
mod waits;

pub(super) const HEAD: &str = "1234567890abcdef1234567890abcdef12345678";

pub(super) fn link() -> PullLink {
    PullLink {
        owner: "o".into(),
        repo: "r".into(),
        number: 9,
    }
}

/// An open pull request with nothing to find.
pub(super) fn ready() -> PullSnapshot {
    PullSnapshot {
        number: 9,
        node_id: "N".into(),
        state: PullState::Open,
        merge_commit: None,
        draft: false,
        body: "## Why\n\nCloses #5\n".into(),
        base_ref: "main".into(),
        head_ref: "feat/issue-5".into(),
        head_sha: HEAD.into(),
        default_branch: "main".into(),
        cross_repository: false,
        mergeable: Some(true),
        mergeable_state: "clean".into(),
        checks: vec![check("Required", Some("success"))],
        required_checks: vec!["Required".into()],
        verdicts: vec![verdict(true, HEAD, 1)],
        threads: Vec::new(),
        queue: QueueFact {
            has_queue: true,
            entry: None,
        },
        codex: None,
        problems: Vec::new(),
        threads_unread: Vec::new(),
    }
}

pub(super) fn check(name: &str, conclusion: Option<&str>) -> CheckFact {
    CheckFact {
        name: name.into(),
        completed: true,
        conclusion: conclusion.map(Into::into),
    }
}

/// A verdict on `head`, submitted at second `second`.
pub(super) fn verdict(pass: bool, head: &str, second: u32) -> VerdictFact {
    VerdictFact {
        pass,
        head: head.into(),
        at: format!("2026-01-01T00:00:{second:02}Z"),
    }
}

/// An unresolved thread.
pub(super) fn thread(url: &str, bot: bool, body: &str) -> ThreadFact {
    ThreadFact {
        id: url.into(),
        resolved: false,
        url: url.into(),
        first_comment_id: 1,
        first_author: if bot { "codex[bot]" } else { "someone" }.into(),
        first_is_bot: bot,
        first_body: body.into(),
        bodies: vec![body.into()],
    }
}

pub(super) fn judge(snapshot: &PullSnapshot) -> Verdict {
    evaluate(snapshot, &config::default(), &link())
}

pub(super) fn codes(verdict: &Verdict) -> Vec<&'static str> {
    verdict.findings.iter().map(|f| f.code.as_str()).collect()
}

#[test]
fn evaluate_a_clean_pull_request_is_ready() {
    let verdict = judge(&ready());
    assert_eq!(verdict.state, State::Ready);
    assert_eq!(verdict.findings, []);
    assert_eq!(
        verdict.next,
        "taskrock pr merge https://github.com/o/r/pull/9"
    );
    assert_eq!(verdict.head, HEAD);
}

#[test]
fn evaluate_a_merged_pull_request_has_no_findings() {
    let mut snapshot = ready();
    snapshot.state = PullState::Merged;
    snapshot.merge_commit = Some("abc123".into());
    snapshot.draft = true;
    snapshot.verdicts.clear();
    let verdict = judge(&snapshot);
    assert_eq!(verdict.state, State::Merged);
    assert_eq!(verdict.findings, []);
    assert_eq!(verdict.next, "merged as abc123");
}

#[test]
fn evaluate_a_closed_pull_request_gives_only_pr_not_open() {
    let mut snapshot = ready();
    snapshot.state = PullState::Closed;
    snapshot.draft = true;
    snapshot.verdicts.clear();
    let verdict = judge(&snapshot);
    assert_eq!(verdict.state, State::Blocked);
    assert_eq!(codes(&verdict), ["pr_not_open"]);
}
