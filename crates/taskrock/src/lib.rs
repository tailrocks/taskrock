//! taskrock: plan and run agent work on GitHub Issues and Projects.

pub mod claim;
pub mod cli;
pub mod draft;
pub mod error;
pub mod fields;
pub mod gh;
pub mod github;
pub mod link;
pub mod ready;

use crate::gh::Gh;

/// Run the command line of taskrock.
///
/// # Errors
///
/// Returns [`error::Error::NotBuilt`] while no command is built yet.
pub fn run(cli: &cli::Cli, gh: &dyn Gh) -> Result<(), error::Error> {
    let name = match &cli.command {
        cli::Command::Check { draft } => {
            let text = draft::check::run(draft)?;
            println!("{text}");
            return Ok(());
        }
        cli::Command::Apply { .. } => "apply",
        cli::Command::Ready { link, json } => {
            let text = ready::run(gh, link, *json)?;
            println!("{text}");
            return Ok(());
        }
        cli::Command::Claim { link } => {
            println!("{}", claim::claim(gh, link)?);
            return Ok(());
        }
        cli::Command::Release { .. } => "release",
    };
    Err(error::Error::NotBuilt(name))
}
