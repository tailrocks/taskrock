//! taskrock: plan and run agent work on GitHub Issues and Projects.

pub mod cli;
pub mod error;
pub mod fields;
pub mod gh;
pub mod github;
pub mod link;

use crate::gh::Gh;

/// Run the command line of taskrock.
///
/// # Errors
///
/// Returns [`error::Error::NotBuilt`] while no command is built yet.
pub fn run(cli: &cli::Cli, gh: &dyn Gh) -> Result<(), error::Error> {
    let _ = gh;
    let name = match &cli.command {
        cli::Command::Check { .. } => "check",
        cli::Command::Apply { .. } => "apply",
        cli::Command::Ready { .. } => "ready",
        cli::Command::Claim { .. } => "claim",
        cli::Command::Release { .. } => "release",
    };
    Err(error::Error::NotBuilt(name))
}
