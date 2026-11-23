//! The draft issues that already exist on GitHub.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::apply::Created;
use crate::apply::marker::marker_key;
use crate::apply::split_repo;
use crate::draft::Draft;
use crate::error::Error;
use crate::gh::Gh;
use crate::github::{get, get_all, is_status};
use crate::link::IssueLink;

#[cfg(test)]
mod tests;

/// How many missing answers in a row end the look past the listing.
const MISSING_LIMIT: u32 = 3;

/// The draft issues that already exist, by key.
///
/// # Errors
///
/// Returns [`Error::Gh`] when `gh` fails, [`Error::Json`] when an entry has
/// no numeric `number` or `id`.
pub(super) fn find_existing(
    gh: &dyn Gh,
    draft: &Draft,
) -> Result<BTreeMap<String, Created>, Error> {
    let mut found = BTreeMap::new();
    for repo in draft_repos(draft) {
        let highest = scan_listing(gh, draft, &repo, &mut found)?;
        look_past_listing(gh, draft, &repo, highest, &mut found)?;
    }
    Ok(found)
}

/// Every repository the issues use, in first-use order.
fn draft_repos(draft: &Draft) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut repos = Vec::new();
    for issue in &draft.issues {
        if seen.insert(issue.repo.clone()) {
            repos.push(issue.repo.clone());
        }
    }
    repos
}

/// Records the marked issues of the listing. Returns its highest number.
fn scan_listing(
    gh: &dyn Gh,
    draft: &Draft,
    repo: &str,
    found: &mut BTreeMap<String, Created>,
) -> Result<u64, Error> {
    let mut highest = 0;
    let path = format!("repos/{repo}/issues?state=all&per_page=100");
    for value in get_all(gh, &path)? {
        if let Some(number) = value["number"].as_u64() {
            highest = highest.max(number);
        }
        record(draft, repo, &value, found)?;
    }
    Ok(highest)
}

/// Reads the issues past the listing: a new issue appears in it late.
/// Stops after [`MISSING_LIMIT`] answers in a row are HTTP 404.
fn look_past_listing(
    gh: &dyn Gh,
    draft: &Draft,
    repo: &str,
    highest: u64,
    found: &mut BTreeMap<String, Created>,
) -> Result<(), Error> {
    let mut number = highest + 1;
    let mut missing = 0;
    while missing < MISSING_LIMIT {
        match get(gh, &format!("repos/{repo}/issues/{number}")) {
            Ok(value) => {
                missing = 0;
                record(draft, repo, &value, found)?;
            }
            Err(error) if is_status(&error, 404) => missing += 1,
            Err(error) => return Err(error),
        }
        number += 1;
    }
    Ok(())
}

/// Records `value` in `found` when its body holds a marker of the plan.
fn record(
    draft: &Draft,
    repo: &str,
    value: &Value,
    found: &mut BTreeMap<String, Created>,
) -> Result<(), Error> {
    if value.get("pull_request").is_some() {
        return Ok(());
    }
    let Some(key) = value["body"]
        .as_str()
        .and_then(|body| marker_key(body, &draft.plan.id))
    else {
        return Ok(());
    };
    let number = value["number"]
        .as_u64()
        .ok_or_else(|| Error::Json("number".to_owned()))?;
    let id = value["id"]
        .as_u64()
        .ok_or_else(|| Error::Json("id".to_owned()))?;
    let (owner, name) = split_repo(repo)?;
    found.insert(
        key,
        Created {
            link: IssueLink {
                owner,
                repo: name,
                number,
            },
            id,
        },
    );
    Ok(())
}
