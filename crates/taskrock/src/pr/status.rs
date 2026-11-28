//! `taskrock pr status`.

use crate::config;
use crate::error::Error;
use crate::gh::Gh;
use crate::link::PullLink;
use crate::pr::snapshot::{self, PullSnapshot};
use crate::pr::{EXIT_BLOCKED, EXIT_DONE, EXIT_WAITING};
use evaluate::{State, Verdict, evaluate};

pub mod code;
pub mod evaluate;
pub mod render;

/// Read the pull request and judge it. `pr merge` uses it as its gate.
///
/// # Errors
///
/// Returns the error of the config load or of the read of the pull request.
pub fn verdict(gh: &dyn Gh, link: &PullLink) -> Result<(PullSnapshot, Verdict), Error> {
    let config = config::load::load(gh, &link.owner)?;
    let snapshot = snapshot::read(gh, link)?;
    let judged = evaluate(&snapshot, &config, link);
    Ok((snapshot, judged))
}

/// The text of the verdict, or its JSON, and its exit code.
fn outcome(link: &PullLink, snapshot: &PullSnapshot, judged: &Verdict, json: bool) -> (String, u8) {
    let out = if json {
        render::json(link, snapshot, judged)
    } else {
        render::text(link, snapshot, judged)
    };
    (out, exit_code(judged.state))
}

/// The exit code of a state: done, waiting or blocked.
#[must_use]
pub const fn exit_code(state: State) -> u8 {
    match state {
        State::Ready | State::Merged => EXIT_DONE,
        State::Waiting => EXIT_WAITING,
        State::Blocked => EXIT_BLOCKED,
    }
}

/// Run `taskrock pr status`: print the verdict and return its exit code.
///
/// # Errors
///
/// Returns the errors of [`verdict`].
pub fn run(gh: &dyn Gh, link: &PullLink, json: bool) -> Result<u8, Error> {
    let (snapshot, judged) = verdict(gh, link)?;
    let (out, code) = outcome(link, &snapshot, &judged, json);
    println!("{out}");
    Ok(code)
}

#[cfg(test)]
mod tests;
