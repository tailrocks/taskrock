//! `taskrock edit`: replace the body of an issue after checking it.

use std::path::Path;

use crate::config::load::load;
use crate::error::Error;
use crate::format;
use crate::gh::Gh;
use crate::github::items::{Item, project_items, project_of};
use crate::github::{get, send};
use crate::link::IssueLink;

#[cfg(test)]
mod support;

#[cfg(test)]
mod tests;

const FROZEN: [&str; 3] = ["Outcome", "Decisions already made", "Out of scope"];
const MARKER: &str = "<!-- taskrock:";
const AGENT_LINE: &str = "_Written by an agent._";

/// Replace the body of the issue at `link` with the text of `file`.
///
/// The body is checked against the format of the issue type: the `file`
/// sections while the status is Triage or empty, the `build` sections after.
/// The owner-task format is used when Agent is `Owner`. Once the status is
/// In Progress or Done, or the issue is closed, the sections `Outcome`,
/// `Decisions already made` and `Out of scope` must stay as they are, unless
/// `replan` is true. A replan posts a comment that names the changed
/// sections. The lines of the old body that start with `<!-- taskrock:` are
/// kept.
///
/// Returns `edited <url>`.
///
/// # Errors
///
/// Returns [`Error::Link`] when `link` is not an issue link,
/// [`Error::Draft`] when the file cannot be read or starts with front matter
/// (before any network call), [`Error::Project`] when the issue is not an
/// item of its project, [`Error::Refused`] when the type is not in the
/// config, the body breaks the format or a frozen section changes without
/// `replan`, and [`Error::Gh`], [`Error::GraphQl`], [`Error::Json`] or [`Error::Config`] when
/// GitHub fails or answers badly.
pub fn run(gh: &dyn Gh, link: &str, file: &Path, replan: bool) -> Result<String, Error> {
    let issue = link.parse::<IssueLink>()?;
    let text = read_body(file)?;
    let data = get(gh, &issue.api_path())?;
    let old = data["body"].as_str().unwrap_or_default();
    let project = project_of(gh, &issue)?;
    let items = project_items(gh, &project)?;
    let item = items
        .iter()
        .find(|item| item.link == issue)
        .ok_or_else(|| {
            Error::Project(format!(
                "the issue is not an item of its project: {}",
                issue.url()
            ))
        })?;
    let kind = data
        .pointer("/type/name")
        .and_then(serde_json::Value::as_str);
    check_format(gh, &issue, item, kind, &text)?;
    let frozen =
        data["state"] != "open" || matches!(item.status.as_deref(), Some("In Progress" | "Done"));
    let changed = if frozen {
        changed_sections(old, &text)
    } else {
        Vec::new()
    };
    if let Some(name) = changed.first().filter(|_| !replan) {
        return Err(Error::Refused(format!(
            "the section ## {name} is frozen after In Progress. Use --replan"
        )));
    }
    let body = keep_markers(old, &text);
    send(
        gh,
        "PATCH",
        &issue.api_path(),
        &serde_json::json!({ "body": body }),
    )?;
    if !changed.is_empty() {
        let comment = format!(
            "{AGENT_LINE}\n\nReplanned: the sections {} changed.",
            changed.join(", ")
        );
        let path = format!("{}/comments", issue.api_path());
        send(gh, "POST", &path, &serde_json::json!({ "body": comment }))?;
    }
    Ok(format!("edited {}", issue.url()))
}

fn read_body(file: &Path) -> Result<String, Error> {
    let name = file.display().to_string();
    let text = std::fs::read_to_string(file).map_err(|error| Error::Draft {
        file: name.clone(),
        message: error.to_string(),
    })?;
    if text.lines().next().is_some_and(|line| line == "+++") {
        return Err(Error::Draft {
            file: name,
            message: "edit takes a body without front matter".to_owned(),
        });
    }
    Ok(text)
}

fn check_format(
    gh: &dyn Gh,
    issue: &IssueLink,
    item: &Item,
    kind: Option<&str>,
    body: &str,
) -> Result<(), Error> {
    let config = load(gh, &issue.owner)?;
    let name = kind.unwrap_or("none");
    let Some(format) = config.format(name, item.routing.is_owner()) else {
        return Err(Error::Refused(format!(
            "the config has no issue type {name}"
        )));
    };
    let sections = match item.status.as_deref() {
        None | Some("Triage") => &format.file,
        Some(_) => &format.build,
    };
    let problems = format::problems(body, sections, &format.checklist);
    if problems.is_empty() {
        return Ok(());
    }
    Err(Error::Refused(format!(
        "the body breaks the {name} format: {}",
        problems.join("; ")
    )))
}

fn changed_sections(old: &str, new: &str) -> Vec<String> {
    FROZEN
        .iter()
        .filter(|name| format::section(old, name) != format::section(new, name))
        .map(|name| (*name).to_owned())
        .collect()
}

fn keep_markers(old: &str, new: &str) -> String {
    let missing: Vec<&str> = old
        .lines()
        .filter(|line| line.starts_with(MARKER) && !new.lines().any(|kept| kept == *line))
        .collect();
    if missing.is_empty() {
        return new.to_owned();
    }
    format!("{}\n\n{}", new.trim_end(), missing.join("\n"))
}
