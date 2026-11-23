//! The milestone numbers of the draft's repositories.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use crate::draft::Draft;
use crate::error::Error;
use crate::gh::Gh;
use crate::github::{get_all, send};

#[cfg(test)]
mod tests;

/// The milestone number by repository and title. Creates a milestone the
/// repository lacks, once per title, unless `dry_run` skips every `POST`.
///
/// # Errors
///
/// Returns [`Error::Gh`] when `gh` fails, [`Error::Json`] when an entry has
/// no numeric `number` or no string `title`.
pub(super) fn milestone_numbers(
    gh: &dyn Gh,
    draft: &Draft,
    dry_run: bool,
) -> Result<BTreeMap<(String, String), u64>, Error> {
    let mut numbers = BTreeMap::new();
    for repo in draft_repos(draft) {
        let have = listed(gh, &repo)?;
        for title in wanted_titles(draft, &repo) {
            if let Some(number) = have.get(&title) {
                numbers.insert((repo.clone(), title), *number);
            } else if !dry_run {
                let number = create(gh, &repo, &title, description(draft, &title))?;
                numbers.insert((repo.clone(), title), number);
            }
        }
    }
    Ok(numbers)
}

/// Every repository the draft uses, in first-use order.
fn draft_repos(draft: &Draft) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut repos = Vec::new();
    for repo in std::iter::once(&draft.plan.repo).chain(draft.issues.iter().map(|i| &i.repo)) {
        if seen.insert(repo.clone()) {
            repos.push(repo.clone());
        }
    }
    repos
}

/// The milestone titles the issues of `repo` use, in first-use order.
fn wanted_titles(draft: &Draft, repo: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut titles = Vec::new();
    for issue in &draft.issues {
        if let Some(title) = &issue.milestone
            && issue.repo == repo
            && seen.insert(title.clone())
        {
            titles.push(title.clone());
        }
    }
    titles
}

/// The milestone numbers `repo` already has, by title.
fn listed(gh: &dyn Gh, repo: &str) -> Result<BTreeMap<String, u64>, Error> {
    let mut have = BTreeMap::new();
    let path = format!("repos/{repo}/milestones?state=all&per_page=100");
    for value in get_all(gh, &path)? {
        let title = value["title"]
            .as_str()
            .ok_or_else(|| Error::Json("title".to_owned()))?;
        let number = value["number"]
            .as_u64()
            .ok_or_else(|| Error::Json("number".to_owned()))?;
        have.insert(title.to_owned(), number);
    }
    Ok(have)
}

/// The description the plan gives `title`, if it gives one.
fn description(draft: &Draft, title: &str) -> Option<String> {
    draft
        .plan
        .milestones
        .iter()
        .find(|milestone| milestone.title == title)
        .and_then(|milestone| milestone.description.clone())
}

/// Creates the milestone and returns its number.
fn create(gh: &dyn Gh, repo: &str, title: &str, description: Option<String>) -> Result<u64, Error> {
    let mut body = Map::new();
    body.insert("title".to_owned(), Value::String(title.to_owned()));
    if let Some(text) = description {
        body.insert("description".to_owned(), Value::String(text));
    }
    let answer = send(
        gh,
        "POST",
        &format!("repos/{repo}/milestones"),
        &Value::Object(body),
    )?;
    answer["number"]
        .as_u64()
        .ok_or_else(|| Error::Json("number".to_owned()))
}
