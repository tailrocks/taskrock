//! taskrock: plan and run agent work on GitHub Issues and Projects.

pub mod claim;
pub mod cli;
pub mod draft;
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
    let name = match &cli.command {
        cli::Command::Check { .. } => "check",
        cli::Command::Apply { .. } => "apply",
        cli::Command::Ready { .. } => "ready",
        cli::Command::Claim { link } => {
            println!("{}", claim::claim(gh, link)?);
            return Ok(());
        }
        cli::Command::Release { .. } => "release",
    };
    Err(error::Error::NotBuilt(name))
}
