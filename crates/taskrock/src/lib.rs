//! taskrock: plan and run agent work on GitHub Issues and Projects.

pub mod apply;
pub mod claim;
pub mod cli;
pub mod config;
pub mod draft;
pub mod edit;
pub mod error;
pub mod fields;
pub mod format;
pub mod gh;
pub mod github;
pub mod handoff;
pub mod link;
pub mod projects;
pub mod ready;
pub mod setup;
pub mod stack;
pub mod template;

use std::path::Path;

use crate::gh::Gh;

/// Run the command line of taskrock.
///
/// # Errors
///
/// Returns the error of the command that runs.
pub fn run(cli: &cli::Cli, gh: &dyn Gh) -> Result<(), error::Error> {
    match &cli.command {
        cli::Command::Check {
            path,
            coverage,
            config,
        } => {
            let text = check(path, *coverage, config.as_deref())?;
            println!("{text}");
            Ok(())
        }
        cli::Command::Apply { draft, dry_run } => {
            let options = apply::Options {
                dry_run: *dry_run,
                pause: std::time::Duration::from_secs(1),
                poll: std::time::Duration::from_secs(5),
            };
            let text = apply::run(gh, draft, &options)?;
            println!("{text}");
            Ok(())
        }
        cli::Command::Ready { link, json } => {
            let text = ready::run(gh, link, *json)?;
            println!("{text}");
            Ok(())
        }
        cli::Command::Claim { link, stacked } => {
            let text = if *stacked {
                claim::claim_stacked(gh, link)?
            } else {
                claim::claim(gh, link)?
            };
            println!("{text}");
            Ok(())
        }
        cli::Command::Release { link, note } => {
            if let Some(note) = note {
                println!("{}", handoff::post(gh, link, note)?);
            }
            println!("{}", claim::release(gh, link)?);
            Ok(())
        }
        cli::Command::Setup {
            owner,
            file,
            dry_run,
            yes,
        } => {
            let options = setup::Options {
                owner: owner.clone(),
                file: file.clone(),
                dry_run: *dry_run,
                yes: *yes,
            };
            println!("{}", setup::run(gh, &options)?);
            Ok(())
        }
        cli::Command::Template {
            kind,
            owner,
            config,
        } => {
            println!(
                "{}",
                template::run(gh, kind, owner.as_deref(), config.as_deref())?
            );
            Ok(())
        }
        cli::Command::Edit { link, file, replan } => {
            println!("{}", edit::run(gh, link, file, *replan)?);
            Ok(())
        }
    }
}

fn check(path: &Path, coverage: bool, config_file: Option<&Path>) -> Result<String, error::Error> {
    if path.is_file() {
        return config::check::run(path);
    }
    let config = match config_file {
        Some(file) => config::parse::read(file)?,
        None => config::default(),
    };
    if coverage {
        draft::coverage::run(path, &config)
    } else {
        draft::check::run(path, &config)
    }
}
