//! `taskrock pr merge`: merge a READY pull request at the head that was judged.

use serde::Serialize;

use crate::error::Error;
use crate::gh::Gh;
use crate::link::PullLink;
use crate::pr::EXIT_WAITING;
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
    retargeted: Vec<u64>,
}

/// The report, the lines of the text output, the refusal of GitHub, and the exit code.
struct Done {
    report: Report,
    lines: Vec<String>,
    refusal: Option<String>,
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
            },
            lines: Vec::new(),
            refusal: None,
            code: 0,
        }
    }

    fn ending(mut self, line: String, code: u8) -> Self {
        self.lines.push(line);
        self.code = code;
        self
    }
}

/// Run `taskrock pr merge` and return its exit code: 0 when the pull request is merged, 10
/// when it waits or is queued, 20 when it is blocked or GitHub refused the merge.
///
/// # Errors
///
/// Returns the errors of [`status::verdict`], [`Error::Refused`] for a pull request from a
/// fork, and the error of a write that GitHub did not refuse by a status of 405, 409 or 422.
pub fn run(gh: &dyn Gh, link: &PullLink, json: bool) -> Result<u8, Error> {
    let done = execute(gh, link)?;
    if json {
        let out = serde_json::to_string_pretty(&done.report)
            .map_err(|error| Error::Json(error.to_string()))?;
        println!("{out}");
        if let Some(message) = &done.refusal {
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
        return Ok(done.ending(format!("already {}", verdict.next), 0));
    }
    if snapshot.cross_repository {
        return Err(Error::Refused("taskrock never merges a fork".to_owned()));
    }
    if verdict.state != State::Ready {
        let text = render::text(link, &snapshot, &verdict);
        let code = status::exit_code(verdict.state);
        return Ok(Done::new(link, head, Outcome::NotReady).ending(text, code));
    }
    let retargeted = write::retarget(gh, link, &snapshot)?;
    let mut done = Done::new(link, head, Outcome::Merged);
    done.lines.extend(
        retargeted
            .iter()
            .map(|number| format!("retargeted #{number} to {}", snapshot.default_branch)),
    );
    done.report.retargeted = retargeted;
    Ok(match write::land(gh, link, &snapshot)? {
        write::Land::Queued(position) => {
            done.report.outcome = Outcome::Queued;
            let text = format!("queued at position {position} (head {})", short(head));
            done.ending(text, EXIT_WAITING)
        }
        write::Land::Merged(commit) => {
            let text = format!(
                "merged as {}",
                commit.as_deref().unwrap_or("an unknown commit")
            );
            done.report.merge_commit = commit;
            done.ending(text, 0)
        }
        write::Land::Refused(message) => {
            done.report.outcome = Outcome::Refused;
            done.refusal = Some(message.clone());
            done.ending(message, crate::pr::EXIT_BLOCKED)
        }
    })
}

/// The first seven characters of a commit.
fn short(sha: &str) -> &str {
    sha.get(..7).unwrap_or(sha)
}
