//! Turn one snapshot of a pull request into a verdict. It is pure and never fails.

use super::code::Code;
use crate::config::Config;
use crate::link::PullLink;
use crate::pr::snapshot::{PullSnapshot, PullState};

mod review;
mod rules;

#[cfg(test)]
mod tests;

/// Whether a finding stops the merge or only waits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Blocker,
    Wait,
}

/// Where the pull request stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Ready,
    Waiting,
    Blocked,
    Merged,
}

/// One reason the pull request is not ready, and what to do about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub code: Code,
    pub detail: String,
    /// What the finding is about: thread links, check names, read problems.
    pub items: Vec<String>,
    pub next: String,
}

/// The state of the pull request, its findings and the next step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub state: State,
    pub head: String,
    /// Blockers first, then waits, each group in [`Code`] order.
    pub findings: Vec<Finding>,
    pub next: String,
}

impl Finding {
    fn new(code: Code, detail: impl Into<String>, next: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
            items: Vec::new(),
            next: next.into(),
        }
    }

    fn items(mut self, items: Vec<String>) -> Self {
        self.items = items;
        self
    }
}

/// Judge `snapshot`. `link` only appears in the `next` texts.
#[must_use]
pub fn evaluate(snapshot: &PullSnapshot, config: &Config, link: &PullLink) -> Verdict {
    let mut findings = match snapshot.state {
        PullState::Merged => return merged(snapshot),
        PullState::Closed => vec![Finding::new(
            Code::PrNotOpen,
            "The pull request is closed and not merged.",
            "ask the owner whether to reopen the pull request",
        )],
        PullState::Open => open(snapshot, config, link),
    };
    findings.sort_by_key(|finding| (finding.code.kind() == Kind::Wait, finding.code));
    let blocked = findings
        .iter()
        .any(|finding| finding.code.kind() == Kind::Blocker);
    let state = if blocked {
        State::Blocked
    } else if findings.is_empty() {
        State::Ready
    } else {
        State::Waiting
    };
    let next = findings.first().map_or_else(
        || format!("taskrock pr merge {}", link.url()),
        |finding| finding.next.clone(),
    );
    Verdict {
        state,
        head: snapshot.head_sha.clone(),
        findings,
        next,
    }
}

fn merged(snapshot: &PullSnapshot) -> Verdict {
    let commit = snapshot
        .merge_commit
        .as_deref()
        .unwrap_or("an unknown commit");
    Verdict {
        state: State::Merged,
        head: snapshot.head_sha.clone(),
        findings: Vec::new(),
        next: format!("merged as {commit}"),
    }
}

fn open(snapshot: &PullSnapshot, config: &Config, link: &PullLink) -> Vec<Finding> {
    let mut out = Vec::new();
    rules::problems(snapshot, &mut out);
    rules::base(snapshot, &mut out);
    rules::mergeability(snapshot, &mut out);
    rules::checks(snapshot, &mut out);
    review::verdicts(snapshot, config, &mut out);
    review::threads(snapshot, link, &mut out);
    rules::draft(snapshot, &mut out);
    rules::closes(snapshot, &mut out);
    review::codex(snapshot, config, link, &mut out);
    rules::queue(snapshot, &mut out);
    out
}
