//! Create the issues of a draft on GitHub.

mod blockers;
mod create;
mod existing;
mod fields;
mod marker;
mod milestones;
mod portfolio;
mod project;
mod status;

use std::collections::BTreeMap;
use std::path::Path;

use crate::config;
use crate::draft::{self};
use crate::error::Error;
use crate::gh::Gh;
use crate::link::IssueLink;

#[cfg(test)]
mod tests;

/// How `apply` creates the issues.
#[derive(Debug, Clone)]
pub struct Options {
    /// Print what would be created and create nothing.
    pub dry_run: bool,
    /// How long to wait after each created issue.
    pub pause: std::time::Duration,
    /// How long to wait between two reads of the project items.
    pub poll: std::time::Duration,
}

/// One issue of the draft on GitHub.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Created {
    /// Where the issue lives.
    pub link: IssueLink,
    /// The database ID of the issue, for sub-issue parents.
    pub id: u64,
    /// Whether this run created the issue. A reported issue found from an
    /// earlier run is set to Triage only when it has no Status.
    pub fresh: bool,
}

/// The draft with the map from key to issue and the lines to print.
pub type ApplyOutput = (draft::Draft, BTreeMap<String, Created>, Vec<String>);

/// Creates the missing issues. Returns the draft, the map from key to issue,
/// and the lines to print.
///
/// # Errors
///
/// Returns [`Error::Draft`] or [`Error::Check`] when the folder is not a
/// clean draft, and [`Error::Gh`], [`Error::Json`] or [`Error::Apply`] when
/// GitHub fails or refuses a creation.
pub fn create_issues(gh: &dyn Gh, folder: &Path, options: &Options) -> Result<ApplyOutput, Error> {
    let draft = draft::parse::read(folder)?;
    let owner = plan_owner(&draft)?;
    let config = config::load::load(gh, &owner)?;
    let findings = draft::check::check(&draft, &config);
    if !findings.is_empty() {
        for finding in &findings {
            println!("{}: {}", finding.file, finding.message);
        }
        return Err(Error::Check(findings.len()));
    }
    status::require_triage(gh, &draft)?;
    portfolio::find_portfolio(gh, &owner, &draft)?;
    let field_ids = fields::field_ids(gh, &owner)?;
    let milestones = milestones::milestone_numbers(gh, &draft, options.dry_run)?;
    let mut found = existing::find_existing(gh, &draft)?;
    let mut lines = Vec::with_capacity(draft.issues.len());
    for issue in create::creation_order(&draft) {
        if let Some(known) = found.get(&issue.key) {
            lines.push(format!(
                "exists  #{} {} {}",
                known.link.number, issue.key, issue.title
            ));
        } else if options.dry_run {
            lines.push(format!("would create {} {}", issue.key, issue.title));
        } else {
            let created = create::create_one(gh, &draft, issue, &found, &field_ids, &milestones)?;
            lines.push(format!(
                "created #{} {} {}",
                created.link.number, issue.key, issue.title
            ));
            found.insert(issue.key.clone(), created);
            std::thread::sleep(options.pause);
        }
    }
    Ok((draft, found, lines))
}

/// The whole command: the issues, their blockers, and their project.
///
/// # Errors
///
/// See [`create_issues`]. Returns [`Error::Gh`] when `gh` fails,
/// [`Error::Json`] when GitHub answers badly, [`Error::Check`] when the
/// folder is not a clean draft, and [`Error::Project`] when a field is
/// missing.
pub fn run(gh: &dyn Gh, folder: &Path, options: &Options) -> Result<String, Error> {
    let (draft, found, mut lines) = create_issues(gh, folder, options)?;
    lines.extend(blockers::link_blockers(
        gh,
        &draft,
        &found,
        options.dry_run,
    )?);
    lines.extend(project::add_to_project(gh, &draft, &found, options)?);
    lines.extend(status::set_statuses(gh, &draft, &found, options)?);
    lines.extend(portfolio::add_to_portfolio(gh, &draft, &found, options)?);
    let blockers: usize = draft
        .issues
        .iter()
        .map(|issue| issue.blocked_by.len())
        .sum();
    lines.push(format!(
        "DONE {} issues, {blockers} blockers",
        draft.issues.len()
    ));
    Ok(lines.join("\n"))
}

/// The owner of the plan's `repo`, which holds the issue fields.
fn plan_owner(draft: &draft::Draft) -> Result<String, Error> {
    split_repo(&draft.plan.repo).map(|(owner, _)| owner)
}

/// Split `owner/name` into its two parts.
fn split_repo(repo: &str) -> Result<(String, String), Error> {
    repo.split_once('/').map_or_else(
        || {
            Err(Error::Draft {
                file: "plan.toml".to_owned(),
                message: "repo must be owner/name".to_owned(),
            })
        },
        |(owner, name)| Ok((owner.to_owned(), name.to_owned())),
    )
}
