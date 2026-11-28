//! `taskrock pr threads`: reply to and resolve the review threads of bots.

use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::gh::Gh;
use crate::github::get;
use crate::link::PullLink;
use crate::pr::classify::{Class, classify};
use crate::pr::snapshot::{PullSnapshot, ThreadFact};
use crate::pr::status;
use crate::pr::{EXIT_BLOCKED, EXIT_DONE};

mod write;

#[cfg(test)]
mod tests;

/// The reply to a P3 thread.
const DEFERRED: &str = "Deferred: severity P3 is low, so it does not block this pull request.";
const CHANGED: &str = "the pull request changed; run the command again";
const SCHEMA: u8 = 1;

/// What the person asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Reply to and resolve every unresolved deferrable thread.
    Defer,
    /// Reply with `reason` to and resolve the bot thread `id`.
    Resolve { id: String, reason: String },
}

/// What a run did. `resolved` holds the link of each thread that is done.
#[derive(Debug, Serialize)]
struct Report {
    schema: u8,
    resolved: Vec<String>,
    failed: Option<String>,
}

/// Run `taskrock pr threads` and return its exit code: done, or blocked by a failed write.
///
/// # Errors
///
/// Returns the errors of [`status::verdict`], [`Error::Refused`] when nothing can be
/// written for `action` or the pull request changed since it was read, and the error of
/// the re-read. A failed write is not an error: the report names it.
pub fn run(gh: &dyn Gh, link: &PullLink, action: &Action, json: bool) -> Result<u8, Error> {
    let (snapshot, _) = status::verdict(gh, link)?;
    let targets = select(&snapshot.threads, action)?;
    if !targets.is_empty() {
        unchanged(gh, link, &snapshot)?;
    }
    let mut report = Report {
        schema: SCHEMA,
        resolved: Vec::new(),
        failed: None,
    };
    for (thread, text) in &targets {
        match write::settle(gh, link, thread, text) {
            Ok(()) => report.resolved.push(thread.url.clone()),
            Err(error) => {
                report.failed = Some(error.to_string());
                break;
            }
        }
    }
    println!("{}", render(&report, targets.len(), json)?);
    Ok(if report.failed.is_some() {
        EXIT_BLOCKED
    } else {
        EXIT_DONE
    })
}

/// The unresolved threads that `action` settles, each with its reply.
fn select<'a>(
    threads: &'a [ThreadFact],
    action: &Action,
) -> Result<Vec<(&'a ThreadFact, String)>, Error> {
    let mut open = threads.iter().filter(|thread| !thread.resolved);
    match action {
        Action::Defer => Ok(open
            .filter(|thread| classify(thread) == Class::Deferrable)
            .map(|thread| (thread, DEFERRED.to_owned()))
            .collect()),
        Action::Resolve { id, reason } => {
            let thread = open
                .find(|thread| thread.id == *id)
                .ok_or_else(|| Error::Refused(format!("no unresolved thread {id}")))?;
            if classify(thread) == Class::Human {
                return Err(Error::Refused("a person resolves human threads".to_owned()));
            }
            Ok(vec![(thread, reason.clone())])
        }
    }
}

#[derive(Deserialize)]
struct Fresh {
    state: String,
    head: Head,
}

#[derive(Deserialize)]
struct Head {
    sha: String,
}

/// Read `pulls/<n>` again: the pull request is still open at the head that was judged.
fn unchanged(gh: &dyn Gh, link: &PullLink, snapshot: &PullSnapshot) -> Result<(), Error> {
    let fresh: Fresh = serde_json::from_value(get(gh, &link.api_path())?)
        .map_err(|error| Error::Json(error.to_string()))?;
    if fresh.state == "open" && fresh.head.sha == snapshot.head_sha {
        Ok(())
    } else {
        Err(Error::Refused(CHANGED.to_owned()))
    }
}

/// The report as text or as JSON. `total` is the number of threads the run set out to settle.
fn render(report: &Report, total: usize, json: bool) -> Result<String, Error> {
    if json {
        return serde_json::to_string_pretty(report)
            .map_err(|error| Error::Json(error.to_string()));
    }
    let mut lines: Vec<String> = report
        .resolved
        .iter()
        .map(|url| format!("resolved {url}"))
        .collect();
    lines.extend(
        report
            .failed
            .iter()
            .map(|message| format!("failed: {message}")),
    );
    lines.push(format!(
        "resolved {} of {total} thread(s)",
        report.resolved.len()
    ));
    Ok(lines.join("\n"))
}
