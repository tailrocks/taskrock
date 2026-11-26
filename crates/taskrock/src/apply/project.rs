//! Adding the issues of a draft to its project.

use std::collections::BTreeMap;

use crate::apply::{Created, Options};
use crate::draft::{Draft, DraftIssue};
use crate::error::Error;
use crate::gh::Gh;
use crate::github::items::{Item, project_items};
use crate::github::send;
use crate::link::{IssueLink, ProjectLink};

#[cfg(test)]
mod tests;

/// How long `apply` waits for GitHub to add the sub-issues by itself.
const WAIT: std::time::Duration = std::time::Duration::from_secs(60);

/// Adds the issues of the draft to its project. Returns the lines to print.
///
/// # Errors
///
/// Returns [`Error::Gh`] when `gh` fails, [`Error::Json`] when an answer
/// has no numeric `number` or `id`, [`Error::Project`] when a field is
/// missing, or the error of [`project_items`].
pub(super) fn add_to_project(
    gh: &dyn Gh,
    draft: &Draft,
    found: &BTreeMap<String, Created>,
    options: &Options,
) -> Result<Vec<String>, Error> {
    if options.dry_run {
        return Ok(draft
            .issues
            .iter()
            .filter(|issue| issue.parent.is_none())
            .map(|issue| format!("would add {} to project", issue.key))
            .collect());
    }
    let mut lines = Vec::new();
    let mut items = project_items(gh, &draft.plan.project)?;
    for issue in parentless_missing(draft, found, &items)? {
        add(
            gh,
            &draft.plan.project,
            &found_link(found, issue)?,
            &mut lines,
        )?;
    }
    for _ in 0..rounds(options.poll) {
        if holds_all(draft, found, &items)? {
            break;
        }
        std::thread::sleep(options.poll);
        items = project_items(gh, &draft.plan.project)?;
    }
    for issue in &draft.issues {
        if !present(found, issue, &items)? {
            add(
                gh,
                &draft.plan.project,
                &found_link(found, issue)?,
                &mut lines,
            )?;
        }
    }
    Ok(lines)
}

/// The draft issues with no parent that the project does not hold yet.
fn parentless_missing<'a>(
    draft: &'a Draft,
    found: &BTreeMap<String, Created>,
    items: &[Item],
) -> Result<Vec<&'a DraftIssue>, Error> {
    let mut missing = Vec::new();
    for issue in &draft.issues {
        if issue.parent.is_none() && !present(found, issue, items)? {
            missing.push(issue);
        }
    }
    Ok(missing)
}

/// True when every issue of the draft is among the items.
fn holds_all(
    draft: &Draft,
    found: &BTreeMap<String, Created>,
    items: &[Item],
) -> Result<bool, Error> {
    for issue in &draft.issues {
        if !present(found, issue, items)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// True when `issue` is among the items.
fn present(
    found: &BTreeMap<String, Created>,
    issue: &DraftIssue,
    items: &[Item],
) -> Result<bool, Error> {
    let link = found_link(found, issue)?;
    Ok(items.iter().any(|item| item.link == link))
}

/// The link of `issue` on GitHub.
fn found_link(found: &BTreeMap<String, Created>, issue: &DraftIssue) -> Result<IssueLink, Error> {
    found
        .get(&issue.key)
        .map(|known| known.link.clone())
        .ok_or_else(|| Error::Json("project".to_owned()))
}

/// Adds `link` to `project` and records the printed line.
pub(super) fn add(
    gh: &dyn Gh,
    project: &ProjectLink,
    link: &IssueLink,
    lines: &mut Vec<String>,
) -> Result<(), Error> {
    let body = serde_json::json!({
        "type": "Issue",
        "owner": link.owner,
        "repo": link.repo,
        "number": link.number,
    });
    send(gh, "POST", &format!("{}/items", project.api_path()), &body)?;
    lines.push(format!("project: added #{}", link.number));
    Ok(())
}

/// How many times the items are read again: every `poll` for [`WAIT`].
fn rounds(poll: std::time::Duration) -> u64 {
    if poll.is_zero() {
        1
    } else {
        WAIT.as_secs() / poll.as_secs().max(1)
    }
}
