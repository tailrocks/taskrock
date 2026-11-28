//! The text and the JSON of a verdict.

use serde::Serialize;

use super::evaluate::{Finding, Kind, State, Verdict};
use crate::link::PullLink;
use crate::pr::classify::classify;
use crate::pr::severity;
use crate::pr::snapshot::PullSnapshot;

#[cfg(test)]
mod json_tests;
#[cfg(test)]
mod tests;

/// The most items that a line of text lists.
const ITEM_CAP: usize = 5;

/// The version of the JSON shape.
const SCHEMA: u8 = 1;

const fn state_name(state: State) -> &'static str {
    match state {
        State::Ready => "READY",
        State::Waiting => "WAITING",
        State::Blocked => "BLOCKED",
        State::Merged => "MERGED",
    }
}

const fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Blocker => "blocker",
        Kind::Wait => "wait",
    }
}

/// The verdict for a person: one line, then two lines for each finding.
#[must_use]
pub fn text(link: &PullLink, _snapshot: &PullSnapshot, verdict: &Verdict) -> String {
    let first = format!(
        "{}/{}#{}: {} (head {}) -> {}",
        link.owner,
        link.repo,
        link.number,
        state_name(verdict.state),
        verdict.head.get(..7).unwrap_or(&verdict.head),
        verdict.next
    );
    let lines = verdict.findings.iter().flat_map(|finding| {
        [
            format!(
                "  [{}] {} ({}): {}",
                kind_name(finding.code.kind()),
                finding.code.as_str(),
                finding.code.owner(),
                detail(finding)
            ),
            format!("      next: {}", finding.next),
        ]
    });
    std::iter::once(first)
        .chain(lines)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The detail, then the items: at most [`ITEM_CAP`], then `, and <k> more`.
fn detail(finding: &Finding) -> String {
    let mut items: Vec<String> = finding.items.iter().take(ITEM_CAP).cloned().collect();
    if let Some(more) = finding.items.len().checked_sub(ITEM_CAP).filter(|k| *k > 0) {
        items.push(format!("and {more} more"));
    }
    if items.is_empty() {
        finding.detail.clone()
    } else {
        format!("{} {}", finding.detail, items.join(", "))
    }
}

#[derive(Serialize)]
struct Report<'a> {
    schema: u8,
    repo: String,
    number: u64,
    head: &'a str,
    state: &'static str,
    next: &'a str,
    findings: Vec<FindingJson<'a>>,
    threads: Vec<ThreadJson<'a>>,
}

#[derive(Serialize)]
struct FindingJson<'a> {
    code: &'static str,
    kind: &'static str,
    owner: &'static str,
    detail: &'a str,
    next: &'a str,
    items: &'a [String],
}

#[derive(Serialize)]
struct ThreadJson<'a> {
    id: &'a str,
    class: &'static str,
    /// `P0` to `P3`, or `null` when the comment has no readable badge.
    severity: Option<String>,
    author: &'a str,
    url: &'a str,
}

/// The verdict for a program: pretty-printed JSON with every item and every unresolved thread.
#[must_use]
pub fn json(link: &PullLink, snapshot: &PullSnapshot, verdict: &Verdict) -> String {
    let report = Report {
        schema: SCHEMA,
        repo: format!("{}/{}", link.owner, link.repo),
        number: link.number,
        head: &verdict.head,
        state: state_name(verdict.state),
        next: &verdict.next,
        findings: verdict
            .findings
            .iter()
            .map(|finding| FindingJson {
                code: finding.code.as_str(),
                kind: kind_name(finding.code.kind()),
                owner: finding.code.owner(),
                detail: &finding.detail,
                next: &finding.next,
                items: &finding.items,
            })
            .collect(),
        threads: snapshot
            .threads
            .iter()
            .filter(|thread| !thread.resolved)
            .map(|thread| ThreadJson {
                id: &thread.id,
                class: classify(thread).as_str(),
                severity: severity::parse(&thread.first_body).map(|s| s.to_string()),
                author: &thread.first_author,
                url: &thread.url,
            })
            .collect(),
    };
    // Plain structs with string keys: serialization cannot fail.
    serde_json::to_string_pretty(&report).unwrap_or_default()
}
