//! Parse a draft folder: `plan.toml` and one issue per Markdown file.

use std::path::Path;
use std::str::FromStr as _;

use serde::Deserialize;

use crate::draft::{Draft, DraftIssue, Milestone, Plan};
use crate::error::Error;
use crate::fields::{ROUTING, Routing};
use crate::link::{Link, ProjectLink};

#[cfg(test)]
mod tests;

/// The plan file inside a draft folder.
const PLAN_FILE: &str = "plan.toml";

/// The line that opens and closes the front matter of an issue file.
const DELIMITER: &str = "+++";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPlan {
    id: String,
    repo: String,
    project: String,
    portfolio: Option<String>,
    #[serde(default)]
    milestones: Vec<RawMilestone>,
    milestone: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMilestone {
    title: String,
    description: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawIssue {
    key: String,
    title: String,
    #[serde(rename = "type")]
    kind: String,
    repo: Option<String>,
    parent: Option<String>,
    #[serde(default)]
    blocked_by: Vec<String>,
    milestone: Option<String>,
    agent: Option<String>,
    runtime: Option<String>,
    model: Option<String>,
    effort: Option<String>,
    review_agent: Option<String>,
    review_runtime: Option<String>,
    review_model: Option<String>,
    review_effort: Option<String>,
}

/// Parse the text of `plan.toml`.
///
/// # Errors
///
/// Returns [`Error::Draft`] when the text is not a valid plan.
pub fn parse_plan(text: &str) -> Result<Plan, Error> {
    let raw: RawPlan = toml::from_str(text).map_err(|error| draft_error(PLAN_FILE, error))?;
    check_name(PLAN_FILE, "id", &raw.id)?;
    check_repo(PLAN_FILE, &raw.repo)?;
    if let Some(portfolio) = &raw.portfolio {
        check_name(PLAN_FILE, "portfolio", portfolio)?;
    }
    if raw.milestone.as_deref().is_some_and(str::is_empty) {
        return Err(draft_error(PLAN_FILE, "milestone must not be empty"));
    }
    let milestones = raw
        .milestones
        .into_iter()
        .map(|milestone| Milestone {
            title: milestone.title,
            description: milestone.description,
        })
        .collect();
    Ok(Plan {
        id: raw.id,
        repo: raw.repo,
        project: parse_project_link(&raw.project)?,
        portfolio: raw.portfolio,
        milestones,
        milestone: raw.milestone,
    })
}

/// Parse the text of one issue file.
///
/// # Errors
///
/// Returns [`Error::Draft`] when the text is not a valid issue.
pub fn parse_issue(file: &str, text: &str, default_repo: &str) -> Result<DraftIssue, Error> {
    let (front, body) = split_issue(file, text)?;
    let raw: RawIssue = toml::from_str(&front).map_err(|error| draft_error(file, error))?;
    check_name(file, "key", &raw.key)?;
    if let Some(repo) = &raw.repo {
        check_repo(file, repo)?;
    }
    let routing = routing_of(&raw);
    Ok(DraftIssue {
        file: file.to_owned(),
        key: raw.key,
        title: raw.title,
        kind: raw.kind,
        repo: raw.repo.unwrap_or_else(|| default_repo.to_owned()),
        parent: raw.parent,
        blocked_by: raw.blocked_by,
        milestone: raw.milestone,
        routing,
        body,
    })
}

/// Read `plan.toml` and every `.md` file of the folder, sorted by file name.
///
/// # Errors
///
/// Returns [`Error::Draft`] when the folder cannot be read.
pub fn read(folder: &Path) -> Result<Draft, Error> {
    let mut names = issue_names(folder)?;
    names.sort();
    let plan = read_plan(folder)?;
    let mut issues = Vec::with_capacity(names.len());
    for name in &names {
        let text =
            std::fs::read_to_string(folder.join(name)).map_err(|error| io_error(name, error))?;
        issues.push(parse_issue(name, &text, &plan.repo)?);
    }
    Ok(Draft { plan, issues })
}

fn read_plan(folder: &Path) -> Result<Plan, Error> {
    let path = folder.join(PLAN_FILE);
    match std::fs::read_to_string(&path) {
        Ok(text) => parse_plan(&text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(Error::Draft {
            file: PLAN_FILE.to_owned(),
            message: "plan.toml is missing".to_owned(),
        }),
        Err(error) => Err(draft_error(PLAN_FILE, error)),
    }
}

fn issue_names(folder: &Path) -> Result<Vec<String>, Error> {
    let folder_name = folder.display().to_string();
    let entries = std::fs::read_dir(folder).map_err(|error| io_error(&folder_name, error))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| io_error(&folder_name, error))?;
        let is_file = entry
            .file_type()
            .map_err(|error| io_error(&folder_name, error))?
            .is_file();
        if !is_file {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let is_issue = std::path::Path::new(&name)
            .extension()
            .is_some_and(|extension| extension == "md");
        if is_issue {
            names.push(name);
        }
    }
    Ok(names)
}

fn split_issue(file: &str, text: &str) -> Result<(String, String), Error> {
    let lines: Vec<&str> = text.lines().collect();
    if lines.first() != Some(&DELIMITER) {
        return Err(draft_error(file, "the file must start with a line +++"));
    }
    let Some(second) = lines.iter().skip(1).position(|line| *line == DELIMITER) else {
        return Err(draft_error(file, "the second +++ line is missing"));
    };
    let front = lines[1..=second].join("\n");
    Ok((front, trim_body(&lines[second + 2..])))
}

fn trim_body(lines: &[&str]) -> String {
    let start = lines
        .iter()
        .position(|line| !line.trim().is_empty())
        .unwrap_or(lines.len());
    lines[start..].join("\n").trim_end().to_owned()
}

fn routing_of(raw: &RawIssue) -> Routing {
    let values = [
        raw.agent.as_deref(),
        raw.runtime.as_deref(),
        raw.model.as_deref(),
        raw.effort.as_deref(),
        raw.review_agent.as_deref(),
        raw.review_runtime.as_deref(),
        raw.review_model.as_deref(),
        raw.review_effort.as_deref(),
    ];
    let mut routing = Routing::default();
    for (field, value) in ROUTING.into_iter().zip(values) {
        if let Some(value) = value {
            routing.set(field, value);
        }
    }
    routing
}

fn parse_project_link(text: &str) -> Result<ProjectLink, Error> {
    match Link::from_str(text) {
        Ok(Link::Project(project)) => Ok(project),
        Ok(Link::Issue(_)) => Err(draft_error(PLAN_FILE, "expected a project link")),
        Err(error) => Err(draft_error(PLAN_FILE, error)),
    }
}

fn check_name(file: &str, field: &str, value: &str) -> Result<(), Error> {
    if valid_name(value) {
        Ok(())
    } else {
        Err(draft_error(
            file,
            format!("only letters, digits and - are allowed in {field}"),
        ))
    }
}

fn check_repo(file: &str, value: &str) -> Result<(), Error> {
    if valid_repo(value) {
        Ok(())
    } else {
        Err(draft_error(file, "repo must be owner/name"))
    }
}

fn valid_name(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn valid_repo(text: &str) -> bool {
    let mut parts = text.split('/');
    matches!(
        (parts.next(), parts.next(), parts.next()),
        (Some(owner), Some(repo), None) if !owner.is_empty() && !repo.is_empty()
    )
}

fn draft_error(file: &str, message: impl std::fmt::Display) -> Error {
    Error::Draft {
        file: file.to_owned(),
        message: message.to_string(),
    }
}

fn io_error(file: &str, error: std::io::Error) -> Error {
    draft_error(file, error)
}
