//! `taskrock setup`: make GitHub match the organization's `taskrock.toml`.

use std::fmt;
use std::path::PathBuf;

use crate::config::{self, Config};
use crate::error::Error;
use crate::gh::Gh;

mod fields;
mod project_fields;
mod projects;
mod types;

#[cfg(test)]
mod tests;

/// The options of `taskrock setup`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub owner: String,
    pub file: Option<PathBuf>,
    pub dry_run: bool,
    pub yes: bool,
}

/// What a setup step does about one thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Create,
    Update,
    Delete,
    Exists,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Delete => "delete",
            Self::Exists => "exists",
        })
    }
}

/// One line of the setup output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub action: Action,
    pub kind: &'static str,
    pub name: String,
}

impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.action, self.kind, self.name)
    }
}

/// The state of one `taskrock setup` run.
#[expect(
    clippy::struct_field_names,
    reason = "`dry_run` is the name the design gives the field"
)]
pub(crate) struct Run<'a> {
    pub gh: &'a dyn Gh,
    pub owner: String,
    pub dry_run: bool,
    #[expect(dead_code, reason = "step 9 of setup reads it")]
    pub yes: bool,
    pub lines: Vec<String>,
}

impl Run<'_> {
    /// Print the line now and keep it. Called before the change is made.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "the design takes the Change by value"
    )]
    pub(crate) fn say(&mut self, change: Change) {
        self.note(change.to_string());
    }

    /// Print and keep a free note line, for example `extra view Old`.
    pub(crate) fn note(&mut self, text: String) {
        println!("{text}");
        self.lines.push(text);
    }

    /// The number of lines that make a change: not `exists`, not a note.
    fn changes(&self) -> usize {
        let actions = [Action::Create, Action::Update, Action::Delete];
        self.lines
            .iter()
            .filter(|line| {
                actions
                    .iter()
                    .any(|action| line.starts_with(&format!("{action} ")))
            })
            .count()
    }
}

/// Make GitHub match the config.
///
/// # Errors
///
/// Returns [`Error::Check`] when the config breaks a rule, before any write,
/// and the errors of the config and of GitHub.
pub fn run(gh: &dyn Gh, options: &Options) -> Result<String, Error> {
    let config = read_config(gh, options)?;
    let mut run = Run {
        gh,
        owner: options.owner.clone(),
        dry_run: options.dry_run,
        yes: options.yes,
        lines: Vec::new(),
    };
    types::issue_types(&mut run, &config)?;
    fields::routing_fields(&mut run, &config)?;
    let projects = projects::projects(&mut run, &config)?;
    project_fields::project_fields(&mut run, &projects)?;
    Ok(format!("DONE {} changes", run.changes()))
}

fn read_config(gh: &dyn Gh, options: &Options) -> Result<Config, Error> {
    let config = match &options.file {
        Some(file) => config::parse::read(file)?,
        None => config::load::load_required(gh, &options.owner)?,
    };
    let problems = config::check::check(&config);
    if problems.is_empty() {
        return Ok(config);
    }
    for problem in &problems {
        println!("taskrock.toml: {problem}");
    }
    Err(Error::Check(problems.len()))
}
