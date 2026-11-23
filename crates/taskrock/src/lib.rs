//! taskrock: plan and run agent work on GitHub Issues and Projects.

pub mod apply;
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
        cli::Command::Apply { draft, dry_run } => {
            let options = apply::Options {
                dry_run: *dry_run,
                pause: std::time::Duration::from_secs(1),
            };
            let text = apply::run(gh, draft, &options)?;
            println!("{text}");
            return Ok(());
        }
        cli::Command::Ready { link, json } => {
            let text = ready::run(gh, link, *json)?;
            println!("{text}");
            return Ok(());
        }
        cli::Command::Claim { link } => {
            println!("{}", claim::claim(gh, link)?);
            return Ok(());
        }
        cli::Command::Release { link } => {
            println!("{}", claim::release(gh, link)?);
            return Ok(());
        }
    };
    Err(error::Error::NotBuilt(name))
}
