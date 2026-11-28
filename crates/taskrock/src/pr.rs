//! The `taskrock pr` commands.

use crate::cli::PrCommand;
use crate::error::Error;
use crate::gh::Gh;
use crate::link::PullLink;

pub mod classify;
pub mod merge;
pub mod severity;
pub mod snapshot;
pub mod status;
pub mod threads;

/// The pull request is done: nothing is left to do.
pub const EXIT_DONE: u8 = 0;
/// The pull request waits for something that will change on its own.
pub const EXIT_WAITING: u8 = 10;
/// The pull request is blocked until someone acts.
pub const EXIT_BLOCKED: u8 = 20;

/// Run one `taskrock pr` command and return its exit code.
///
/// # Errors
///
/// Returns [`Error::Link`] when the link is not a pull request link, and the
/// error of the command that runs.
pub fn run(gh: &dyn Gh, command: &PrCommand) -> Result<u8, Error> {
    match command {
        PrCommand::Status { link, json } => status::run(gh, &link.parse::<PullLink>()?, *json),
        PrCommand::Threads {
            link,
            json,
            defer,
            resolve,
            reason,
        } => {
            let action = match (defer, resolve, reason) {
                (true, None, None) => threads::Action::Defer,
                (false, Some(id), Some(reason)) => threads::Action::Resolve {
                    id: id.clone(),
                    reason: reason.clone(),
                },
                _ => {
                    return Err(Error::Refused(
                        "pass --defer, or --resolve with --reason".to_owned(),
                    ));
                }
            };
            threads::run(gh, &link.parse::<PullLink>()?, &action, *json)
        }
        PrCommand::Merge { link, .. } => merge::run(gh, &link.parse::<PullLink>()?),
    }
}
