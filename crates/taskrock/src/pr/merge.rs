//! `taskrock pr merge`: merge a READY pull request at the head that was judged.

use serde::Serialize;

use crate::error::Error;
use crate::gh::Gh;
use crate::link::PullLink;
use crate::pr::EXIT_WAITING;
use crate::pr::snapshot::PullSnapshot;
use crate::pr::status::evaluate::State;
use crate::pr::status::{self, render};

mod write;

#[cfg(test)]
mod tests;

const SCHEMA: u8 = 1;

/// What a run did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Outcome {
    Merged,
    Queued,
    AlreadyMerged,
    NotReady,
    Refused,
}

/// The report of a run. `--json` prints it.
#[derive(Debug, Serialize)]
struct Report {
    schema: u8,
    repo: String,
    number: u64,
    outcome: Outcome,
    head: String,
    merge_commit: Option<String>,
    /// The open pull requests that were based on the head branch and now point at the branch
    /// the pull request merged into (its base, which is the default branch only for a pull
    /// request that was not stacked). A run writes them only after the pull request is merged:
    /// the outcomes `merged` and `already_merged`. It is empty for `queued`, `refused` and `not_ready`: a queued pull
    /// request retargets its children when `pr merge` runs again on it after it merged.
    retargeted: Vec<u64>,
    /// The children that GitHub would not retarget after the merge landed. Always present,
    /// empty when none failed. The exit code stays 0: the merge landed, and a rerun of `pr
    /// merge` retargets what is left.
    retarget_failures: Vec<write::RetargetFailure>,
    /// The error of the listing of the children, or null. When it is set, the run could not
    /// tell which pull requests were based on the head branch.
    retarget_error: Option<String>,
}

/// The report, the lines of the text output, the refusal of GitHub, and the exit code.
struct Done {
    report: Report,
    lines: Vec<String>,
    refusal: Option<String>,
    /// The lines of the retargets that failed after the merge landed, for stderr with
    /// `--json`. They do not undo the merge.
    failures: Vec<String>,
    code: u8,
}

impl Done {
    fn new(link: &PullLink, head: &str, outcome: Outcome) -> Self {
        Self {
            report: Report {
                schema: SCHEMA,
                repo: format!("{}/{}", link.owner, link.repo),
                number: link.number,
                outcome,
                head: head.to_owned(),
                merge_commit: None,
                retargeted: Vec::new(),
                retarget_failures: Vec::new(),
                retarget_error: None,
            },
            lines: Vec::new(),
            refusal: None,
            failures: Vec::new(),
            code: 0,
        }
    }

    fn ending(mut self, line: String, code: u8) -> Self {
        self.lines.push(line);
        self.code = code;
        self
    }

    /// Point the open pull requests based on the merged head branch at the branch it merged
    /// into. A pull request from a fork has a head branch in another repository, so its name
    /// says nothing about the pull requests here: it retargets none. A failed retarget is a
    /// line, never an error: the merge already landed.
    fn retargeted(mut self, gh: &dyn Gh, link: &PullLink, snapshot: &PullSnapshot) -> Self {
        if snapshot.cross_repository {
            return self;
        }
        let write::Retargeted {
            moved,
            failures,
            list_error,
        } = write::retarget(gh, link, snapshot);
        self.lines.extend(
            moved
                .iter()
                .map(|number| format!("retargeted #{number} to {}", snapshot.base_ref)),
        );
        self.failures = failures
            .iter()
            .map(|failure| {
                format!(
                    "could not retarget #{} to {}: {}",
                    failure.number, snapshot.base_ref, failure.error
                )
            })
            .chain(list_error.iter().map(|error| {
                format!(
                    "could not list the pull requests based on {}: {error}",
                    snapshot.head_ref
                )
            }))
            .collect();
        self.lines.extend(self.failures.iter().cloned());
        self.report.retargeted = moved;
        self.report.retarget_failures = failures;
        self.report.retarget_error = list_error;
        self
    }
}

/// Run `taskrock pr merge` and return its exit code: 0 when the pull request is merged, 10
/// when it waits or is queued, 20 when it is blocked or GitHub refused the merge.
///
/// # Errors
///
/// Returns the errors of [`status::verdict`], [`Error::Refused`] for a pull request from a
/// fork, and the error of a merge or enqueue that GitHub did not refuse by a status of 405,
/// 409 or 422. A retarget that fails after the merge landed is a line of the output, not an
/// error.
pub fn run(gh: &dyn Gh, link: &PullLink, json: bool) -> Result<u8, Error> {
    let done = execute(gh, link)?;
    if json {
        let out = serde_json::to_string_pretty(&done.report)
            .map_err(|error| Error::Json(error.to_string()))?;
        println!("{out}");
        for message in done.refusal.iter().chain(&done.failures) {
            eprintln!("{message}");
        }
    } else {
        println!("{}", done.lines.join("\n"));
    }
    Ok(done.code)
}

fn execute(gh: &dyn Gh, link: &PullLink) -> Result<Done, Error> {
    let (snapshot, verdict) = status::verdict(gh, link)?;
    let head = &snapshot.head_sha;
    if verdict.state == State::Merged {
        let mut done = Done::new(link, head, Outcome::AlreadyMerged);
        done.report.merge_commit.clone_from(&snapshot.merge_commit);
        return Ok(done
            .ending(format!("already {}", verdict.next), 0)
            .retargeted(gh, link, &snapshot));
    }
    if snapshot.cross_repository {
        return Err(Error::Refused("taskrock never merges a fork".to_owned()));
    }
    if verdict.state != State::Ready {
        let text = render::text(link, &snapshot, &verdict);
        let code = status::exit_code(verdict.state);
        return Ok(Done::new(link, head, Outcome::NotReady).ending(text, code));
    }
    let mut done = Done::new(link, head, Outcome::Merged);
    match write::land(gh, link, &snapshot)? {
        write::Land::Queued(position) => {
            done.report.outcome = Outcome::Queued;
            let text = format!(
                "queued at position {position} (head {}); the pull requests based on {} are \
                 retargeted when it merges: run taskrock pr merge {} again then",
                short(head),
                snapshot.head_ref,
                link.url()
            );
            Ok(done.ending(text, EXIT_WAITING))
        }
        write::Land::Merged(commit) => {
            let text = format!(
                "merged as {}",
                commit.as_deref().unwrap_or("an unknown commit")
            );
            done.report.merge_commit = commit;
            Ok(done.ending(text, 0).retargeted(gh, link, &snapshot))
        }
        write::Land::Refused(message) => {
            done.report.outcome = Outcome::Refused;
            done.refusal = Some(message.clone());
            Ok(done.ending(message, crate::pr::EXIT_BLOCKED))
        }
    }
}

/// The first seven characters of a commit.
fn short(sha: &str) -> &str {
    sha.get(..7).unwrap_or(sha)
}
