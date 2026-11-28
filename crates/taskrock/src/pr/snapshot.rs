//! One read of a pull request: every fact that `pr status`, `pr threads` and
//! `pr merge` need, or the reason why a fact could not be read.

use crate::error::Error;
use crate::gh::Gh;
use crate::link::PullLink;

mod graph;
mod rest;

#[cfg(test)]
pub(crate) mod case_tests;
#[cfg(test)]
mod facts_tests;
#[cfg(test)]
mod tests;

/// Where a pull request is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullState {
    Open,
    Closed,
    Merged,
}

/// One check run or commit status of the head commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckFact {
    pub name: String,
    pub completed: bool,
    pub conclusion: Option<String>,
}

/// A `VERDICT: PASS|FAIL head=<sha>` line at the end of a review or a comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerdictFact {
    pub pass: bool,
    pub head: String,
    /// When the review was submitted or the comment was created.
    pub at: String,
}

/// One review thread with the text of its comments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadFact {
    pub id: String,
    pub resolved: bool,
    pub url: String,
    pub first_comment_id: u64,
    pub first_author: String,
    pub first_is_bot: bool,
    pub first_body: String,
    pub bodies: Vec<String>,
}

/// The place of a pull request in the merge queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueEntry {
    pub state: String,
    pub position: Option<u64>,
}

/// Whether the base branch has a merge queue, and where the pull request is in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueFact {
    pub has_queue: bool,
    pub entry: Option<QueueEntry>,
}

/// The state that the Codex review comment records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexFact {
    pub code_sha: String,
    pub code_status: String,
    pub security_sha: String,
    pub security_status: String,
}

/// Everything one read of a pull request found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullSnapshot {
    pub number: u64,
    pub node_id: String,
    pub state: PullState,
    pub merge_commit: Option<String>,
    pub draft: bool,
    pub body: String,
    pub base_ref: String,
    pub head_ref: String,
    pub head_sha: String,
    pub default_branch: String,
    pub cross_repository: bool,
    pub mergeable: Option<bool>,
    pub mergeable_state: String,
    pub checks: Vec<CheckFact>,
    pub required_checks: Vec<String>,
    pub verdicts: Vec<VerdictFact>,
    pub threads: Vec<ThreadFact>,
    pub queue: QueueFact,
    pub codex: Option<CodexFact>,
    /// One sentence for each fact that could not be read in full.
    pub problems: Vec<String>,
    /// The sentences of `problems` that say the review threads were not read in full. Empty
    /// when they were: a thread or a marker can hide behind each of them.
    pub threads_unread: Vec<String>,
}

/// Read the pull request: six REST reads and one GraphQL query.
///
/// # Errors
///
/// Returns the error of the read of `pulls/<n>`. Every other read that fails
/// adds one sentence to `problems` instead.
pub fn read(gh: &dyn Gh, link: &PullLink) -> Result<PullSnapshot, Error> {
    let mut snapshot = rest::pull(gh, link)?;
    let (head, base) = (snapshot.head_sha.clone(), snapshot.base_ref.clone());
    let mut problems = Vec::new();
    let runs = rest::check_runs(gh, link, &head, &mut problems);
    snapshot.checks = settle(runs, "check runs", &mut problems);
    let statuses = rest::statuses(gh, link, &head, &mut problems);
    snapshot
        .checks
        .extend(settle(statuses, "commit statuses", &mut problems));
    let rules = settle(rest::rules(gh, link, &base), "branch rules", &mut problems);
    snapshot.required_checks = rules.required_checks;
    let reviews = settle(rest::reviews(gh, link), "reviews", &mut problems);
    let notes = rest::comments(gh, link, &mut problems);
    let comments = settle(notes, "comments", &mut problems);
    snapshot.verdicts = reviews;
    snapshot.verdicts.extend(comments.verdicts);
    snapshot.verdicts.sort_by(|a, b| a.at.cmp(&b.at));
    snapshot.codex = comments.codex;
    let found = graph::read(gh, link, &base, &mut problems);
    let graph = found.unwrap_or_else(|error| {
        graph::Graph::failed(unreadable("review threads and the merge queue", &error))
    });
    problems.extend(graph.unread.iter().cloned());
    snapshot.threads_unread = graph.unread;
    snapshot.threads = graph.threads;
    snapshot.queue = QueueFact {
        has_queue: graph.has_queue || rules.merge_queue,
        entry: graph.entry,
    };
    snapshot.problems = problems;
    Ok(snapshot)
}

/// The value of a read, or its default and one sentence in `problems`.
fn settle<T: Default>(result: Result<T, Error>, what: &str, problems: &mut Vec<String>) -> T {
    result.unwrap_or_else(|error| {
        problems.push(unreadable(what, &error));
        T::default()
    })
}

fn unreadable(what: &str, error: &Error) -> String {
    format!("The {what} could not be read: {error}.")
}
