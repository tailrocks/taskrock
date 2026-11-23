//! The blocker links between the issues of a draft.

use std::collections::{BTreeMap, BTreeSet};

use crate::apply::Created;
use crate::draft::Draft;
use crate::error::Error;
use crate::gh::Gh;
use crate::github::{get_all, send};
use crate::link::IssueLink;

#[cfg(test)]
mod tests;

/// Links the blockers of the draft. Returns the lines to print.
///
/// # Errors
///
/// Returns [`Error::Gh`] when `gh` fails, [`Error::Json`] when a listed
/// blocker has no numeric `id` or a key is missing from `found`.
pub(super) fn link_blockers(
    gh: &dyn Gh,
    draft: &Draft,
    found: &BTreeMap<String, Created>,
    dry_run: bool,
) -> Result<Vec<String>, Error> {
    let mut lines = Vec::new();
    for issue in &draft.issues {
        if issue.blocked_by.is_empty() {
            continue;
        }
        if dry_run {
            for blocker in &issue.blocked_by {
                lines.push(format!("would block {} by {}", issue.key, blocker));
            }
            continue;
        }
        let known = found
            .get(&issue.key)
            .ok_or_else(|| Error::Json("blocked_by".to_owned()))?;
        let have = existing_ids(gh, &known.link)?;
        for blocker in &issue.blocked_by {
            let other = found
                .get(blocker)
                .ok_or_else(|| Error::Json("blocked_by".to_owned()))?;
            if have.contains(&other.id) {
                continue;
            }
            let body = serde_json::json!({"issue_id": other.id});
            send(gh, "POST", &blocked_path(&known.link), &body)?;
            lines.push(format!(
                "blocked #{} by #{}",
                known.link.number, other.link.number
            ));
        }
    }
    Ok(lines)
}

/// The database IDs of the issues that already block `link`.
fn existing_ids(gh: &dyn Gh, link: &IssueLink) -> Result<BTreeSet<u64>, Error> {
    let mut ids = BTreeSet::new();
    for value in get_all(gh, &format!("{}?per_page=100", blocked_path(link)))? {
        ids.insert(
            value["id"]
                .as_u64()
                .ok_or_else(|| Error::Json("id".to_owned()))?,
        );
    }
    Ok(ids)
}

/// `repos/{owner}/{repo}/issues/{number}/dependencies/blocked_by`.
fn blocked_path(link: &IssueLink) -> String {
    format!("{}/dependencies/blocked_by", link.api_path())
}
