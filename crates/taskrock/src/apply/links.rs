//! The issues that already exist and that the draft links to.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::apply::Created;
use crate::draft::{Draft, ParentMilestones, links};
use crate::error::Error;
use crate::gh::Gh;
use crate::github::{get, is_status};
use crate::link::IssueLink;

#[cfg(test)]
mod flow_tests;
#[cfg(test)]
mod milestone_tests;
#[cfg(test)]
mod order_tests;
#[cfg(test)]
mod tests;

/// How many sub-issues GitHub allows under one parent.
const SUB_ISSUE_LIMIT: u64 = 100;

/// The linked issues of a draft.
#[derive(Debug)]
pub(super) struct Resolved {
    /// The issue behind each link, by link text.
    pub found: BTreeMap<String, Created>,
    /// The milestone title of each link that has one, by link text.
    pub milestones: ParentMilestones,
}

/// Reads each issue the draft uses as `parent` or in `blocked_by` by link.
/// The map key is the link text, which is never a key of the draft.
/// `existing` holds the draft issues already on GitHub: a parent's count
/// has them, so the limit check counts only the others.
///
/// # Errors
///
/// Returns [`Error::Apply`] when a link points at no issue or at a pull
/// request, or when a parent has no room for the draft issues that name it,
/// [`Error::Gh`] when `gh` fails, [`Error::Json`] when the answer has no
/// numeric `id`.
pub(super) fn resolve(
    gh: &dyn Gh,
    draft: &Draft,
    existing: &BTreeMap<String, Created>,
) -> Result<Resolved, Error> {
    let mut resolved = Resolved {
        found: BTreeMap::new(),
        milestones: ParentMilestones::new(),
    };
    for link in links::used(draft) {
        let value = read_issue(gh, &link)?;
        let id = value["id"]
            .as_u64()
            .ok_or_else(|| Error::Json("id".to_owned()))?;
        check_room(draft, existing, &link, &value)?;
        if let Some(title) = value["milestone"]["title"].as_str() {
            resolved.milestones.insert(link.url(), title.to_owned());
        }
        resolved.found.insert(
            link.url(),
            Created {
                link,
                id,
                fresh: false,
            },
        );
    }
    Ok(resolved)
}

/// The issue `link` points at.
fn read_issue(gh: &dyn Gh, link: &IssueLink) -> Result<Value, Error> {
    let value = match get(gh, &link.api_path()) {
        Ok(value) => value,
        Err(error) if is_status(&error, 404) || is_status(&error, 410) => {
            return Err(Error::Apply(format!("no issue {}", link.url())));
        }
        Err(error) => return Err(error),
    };
    if value.get("pull_request").is_some() {
        return Err(Error::Apply(format!(
            "{} is a pull request, not an issue",
            link.url()
        )));
    }
    Ok(value)
}

/// Fails when the parent has `sub_issues_summary.total` sub-issues and the
/// draft issues not yet created that name it do not fit under the limit.
fn check_room(
    draft: &Draft,
    existing: &BTreeMap<String, Created>,
    link: &IssueLink,
    value: &Value,
) -> Result<(), Error> {
    let total = value["sub_issues_summary"]["total"].as_u64().unwrap_or(0);
    let url = link.url();
    let new = draft
        .issues
        .iter()
        .filter(|issue| issue.parent.as_deref() == Some(url.as_str()))
        .filter(|issue| !existing.contains_key(&issue.key))
        .count();
    let new = u64::try_from(new).unwrap_or(u64::MAX);
    if total + new > SUB_ISSUE_LIMIT {
        return Err(Error::Apply(format!(
            "{url} has {total} sub-issues and GitHub allows {SUB_ISSUE_LIMIT}: file under a child Epic of it"
        )));
    }
    Ok(())
}
