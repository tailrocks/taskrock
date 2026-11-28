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

/// Run `taskrock pr status`: print the verdict and return its exit code.
///
/// # Errors
///
/// Returns the errors of [`verdict`].
pub fn run(gh: &dyn Gh, link: &PullLink, json: bool) -> Result<u8, Error> {
    let (snapshot, judged) = verdict(gh, link)?;
    let out = if json {
        render::json(link, &snapshot, &judged)
    } else {
        render::text(link, &snapshot, &judged)
    };
    println!("{out}");
    Ok(match judged.state {
        State::Ready | State::Merged => EXIT_DONE,
        State::Waiting => EXIT_WAITING,
        State::Blocked => EXIT_BLOCKED,
    })
}
