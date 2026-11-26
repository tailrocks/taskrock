//! The bodies of the open issues, and what a ready task body must hold.

use std::collections::{BTreeMap, BTreeSet};

use crate::config::Config;
use crate::error::Error;
use crate::format;
use crate::gh::Gh;
use crate::github::get_all;
use crate::github::items::Item;
use crate::link::IssueLink;

#[cfg(test)]
mod tests;

/// The bodies of the open issues of each repository of the scope, by link.
///
/// One `get_all` per repository: `repos/<owner>/<repo>/issues?state=open&per_page=100`.
/// Entries with a `pull_request` key are skipped.
///
/// # Errors
///
/// Returns [`Error::Gh`] when `gh` fails, [`Error::Json`] when the answer is
/// not JSON.
pub(super) fn bodies(gh: &dyn Gh, scope: &[&Item]) -> Result<BTreeMap<IssueLink, String>, Error> {
    let mut repos = BTreeSet::new();
    for item in scope {
        repos.insert((item.link.owner.clone(), item.link.repo.clone()));
    }
    let mut out = BTreeMap::new();
    for (owner, repo) in repos {
        let path = format!("repos/{owner}/{repo}/issues?state=open&per_page=100");
        for value in get_all(gh, &path)? {
            if value.get("pull_request").is_some() {
                continue;
            }
            let Some(number) = value.get("number").and_then(serde_json::Value::as_u64) else {
                continue;
            };
            let body = value
                .get("body")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let link = IssueLink {
                owner: owner.clone(),
                repo: repo.clone(),
                number,
            };
            out.insert(link, body.to_owned());
        }
    }
    Ok(out)
}

/// The build problems of one candidate: [`format::problems`] with the build
/// and checklist sections of its type (the owner-task format when Agent is
/// Owner). A Feature with children is not a candidate.
pub(super) fn build_problems(item: &Item, body: &str, config: &Config) -> Vec<String> {
    let kind = item.kind.as_deref().unwrap_or_default();
    match config.format(kind, item.routing.is_owner()) {
        Some(found) => format::problems(body, &found.build, &found.checklist),
        None => Vec::new(),
    }
}

/// The repository paths named in inline code spans of the section
/// "Decisions already made": a span with no space, at least one '/', and a
/// '.' in its last part.
pub(super) fn named_files(body: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let Some(text) = format::section(body, "Decisions already made") else {
        return out;
    };
    let mut fenced = false;
    for line in text.lines() {
        if line.starts_with("```") {
            fenced = !fenced;
        } else if !fenced {
            let parts: Vec<&str> = line.split('`').collect();
            let closed = parts.len() - 1 - (parts.len() - 1) % 2;
            for span in parts.iter().take(closed).skip(1).step_by(2) {
                if is_file(span) {
                    out.insert((*span).to_owned());
                }
            }
        }
    }
    out
}

fn is_file(span: &str) -> bool {
    !span.contains(char::is_whitespace)
        && span.contains('/')
        && span
            .rsplit('/')
            .next()
            .is_some_and(|last| last.contains('.'))
}
