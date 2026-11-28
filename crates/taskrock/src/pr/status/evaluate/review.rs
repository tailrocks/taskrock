//! The rules about reviews: verdicts, rounds, threads and the Codex review.

use super::Finding;
use crate::config::Config;
use crate::link::PullLink;
use crate::pr::classify::{Class, classify};
use crate::pr::snapshot::{PullSnapshot, ThreadFact, VerdictFact};
use crate::pr::status::code::Code;

/// The first seven characters of a commit.
fn short(sha: &str) -> &str {
    sha.get(..7).unwrap_or(sha)
}

/// The verdict rules and the round limit. `snapshot.verdicts` is sorted by time.
pub(super) fn verdicts(snapshot: &PullSnapshot, config: &Config, out: &mut Vec<Finding>) {
    let head = short(&snapshot.head_sha);
    let newest: Option<&VerdictFact> = snapshot
        .verdicts
        .iter()
        .rfind(|verdict| verdict.head == snapshot.head_sha);
    match newest {
        Some(verdict) if verdict.pass => {}
        Some(_) => out.push(Finding::new(
            Code::VerdictFailed,
            format!("The newest verdict for {head} is FAIL."),
            "fix the findings of the FAIL verdict and push",
        )),
        None => {
            let older = snapshot
                .verdicts
                .last()
                .map_or_else(String::new, |verdict| {
                    format!(" The newest older verdict is for {}.", short(&verdict.head))
                });
            out.push(Finding::new(
                Code::VerdictMissing,
                format!("There is no verdict for {head}.{older}"),
                format!(
                    "reviewer: review {head} and post VERDICT: PASS|FAIL head={}",
                    snapshot.head_sha
                ),
            ));
        }
    }
    let mut heads: Vec<&str> = snapshot.verdicts.iter().map(|v| v.head.as_str()).collect();
    heads.sort_unstable();
    heads.dedup();
    let passed = newest.is_some_and(|verdict| verdict.pass);
    if heads.len() >= config.review.max_rounds as usize && !passed {
        out.push(Finding::new(
            Code::RoundsExhausted,
            format!(
                "{} heads were reviewed and the limit is {}.",
                heads.len(),
                config.review.max_rounds
            ),
            "owner: decide whether to change the task, merge as it is or close the pull request",
        ));
    }
}

pub(super) fn threads(snapshot: &PullSnapshot, link: &PullLink, out: &mut Vec<Finding>) {
    let url = link.url();
    let of = |class: Class| -> Vec<String> {
        let open = snapshot
            .threads
            .iter()
            .filter(|thread: &&ThreadFact| !thread.resolved);
        open.filter(|thread| classify(thread) == class)
            .map(|thread| thread.url.clone())
            .collect()
    };
    let rows = [
        (
            Class::Blocking,
            Code::ThreadsBlocking,
            format!(
                "reviewer: reproduce it; reproduced goes in the FAIL, otherwise taskrock pr threads {url} --resolve <id> --reason \"<why>\""
            ),
        ),
        (
            Class::Human,
            Code::ThreadsHuman,
            "answer the threads of the people, then resolve them".to_owned(),
        ),
        (
            Class::Deferrable,
            Code::ThreadsDeferrable,
            format!("taskrock pr threads {url} --defer"),
        ),
    ];
    for (class, code, next) in rows {
        let items = of(class);
        if !items.is_empty() {
            let detail = format!("{} unresolved {} thread(s).", items.len(), class.as_str());
            out.push(Finding::new(code, detail, next).items(items));
        }
    }
}

/// Codex has not reviewed the head yet, or is still running. With no Codex
/// comment there is nothing to wait for: the connector posts it when a review starts.
pub(super) fn codex(snapshot: &PullSnapshot, out: &mut Vec<Finding>) {
    let Some(codex) = &snapshot.codex else {
        return;
    };
    let head = &snapshot.head_sha;
    let done = |sha: &str, status: &str| sha == head && status == "completed";
    if !(done(&codex.code_sha, &codex.code_status)
        && done(&codex.security_sha, &codex.security_status))
    {
        out.push(Finding::new(
            Code::BotReviewPending,
            "The Codex code and security review of the head has not completed.",
            format!(
                "wait for the Codex code and security review of {}",
                short(head)
            ),
        ));
    }
}
