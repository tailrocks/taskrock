//! `parent` and `blocked_by` values that are links to issues that already exist.

use std::collections::BTreeSet;

use super::Draft;
use crate::link::IssueLink;

#[cfg(test)]
mod tests;

/// The start of a value that is meant as a link, good or bad.
const PREFIX: &str = "https://";

/// True when the value of `parent` or `blocked_by` is a link, not a key.
#[must_use]
pub fn is_link(value: &str) -> bool {
    value.starts_with(PREFIX)
}

/// The issue a value links to: exactly
/// `https://github.com/<owner>/<repo>/issues/<number>`.
#[must_use]
pub fn parse(value: &str) -> Option<IssueLink> {
    if !value.starts_with("https://github.com/") {
        return None;
    }
    value
        .parse::<IssueLink>()
        .ok()
        .filter(|link| link.url() == value)
}

/// The links the draft uses as `parent` or in `blocked_by`, each once, in
/// first-use order. A bad link is left out: `check` reports it.
#[must_use]
pub fn used(draft: &Draft) -> Vec<IssueLink> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for issue in &draft.issues {
        for value in issue.parent.iter().chain(&issue.blocked_by) {
            if let Some(link) = parse(value)
                && seen.insert(link.clone())
            {
                out.push(link);
            }
        }
    }
    out
}
