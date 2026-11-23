//! Take a task: create its branch and move it to `In Progress`.

use crate::error::Error;
use crate::gh::Gh;
use crate::github::items::{Item, project_items, project_of, set_status, status_field};
use crate::github::{get, is_status, send};
use crate::link::{IssueLink, ProjectLink};

#[cfg(test)]
mod tests;

/// Take the task at `link`: create `feat/issue-<number>` and move the task
/// and its open parents to `In Progress`.
///
/// Returns two lines: `taken <url>` and `branch feat/issue-<number>`.
///
/// # Errors
///
/// Returns [`Error::Link`] when `link` is not an issue link, [`Error::Project`]
/// when the issue is not an item of its project, [`Error::Refused`] when the
/// issue cannot be taken, and [`Error::Gh`] or [`Error::Json`] when GitHub
/// fails or answers badly.
pub fn claim(gh: &dyn Gh, link: &str) -> Result<String, Error> {
    let issue = link.parse::<IssueLink>()?;
    let project = project_of(gh, &issue)?;
    let items = project_items(gh, &project)?;
    let item = find(&items, &issue)?.clone();
    refuse(&item)?;
    create_branch(gh, &issue)?;
    if let Err(error) = mark(gh, &project, &items, &item) {
        delete_branch(gh, &issue);
        return Err(error);
    }
    Ok(format!("taken {}\nbranch {}", issue.url(), issue.branch()))
}

fn find<'a>(items: &'a [Item], issue: &IssueLink) -> Result<&'a Item, Error> {
    items
        .iter()
        .find(|item| item.link == *issue)
        .ok_or_else(|| {
            Error::Project(format!(
                "the issue is not an item of its project: {}",
                issue.url()
            ))
        })
}

fn refuse(item: &Item) -> Result<(), Error> {
    if !item.open {
        return Err(Error::Refused("the issue is closed".to_owned()));
    }
    if !matches!(item.kind.as_deref(), Some("Task" | "Bug")) {
        return Err(Error::Refused(
            "only a Task or a Bug can be taken".to_owned(),
        ));
    }
    if item.children > 0 {
        return Err(Error::Refused(
            "the issue has children. Only an issue with no children is built".to_owned(),
        ));
    }
    if item.routing.is_owner() {
        return Err(Error::Refused("this is the owner's task".to_owned()));
    }
    if item.open_blockers > 0 {
        return Err(Error::Refused(format!(
            "{} open issues block it",
            item.open_blockers
        )));
    }
    match item.status.as_deref() {
        Some("Todo") => Ok(()),
        Some(status) => Err(Error::Refused(format!("the status is {status}, not Todo"))),
        None => Err(Error::Refused("the issue has no status".to_owned())),
    }
}

fn create_branch(gh: &dyn Gh, issue: &IssueLink) -> Result<(), Error> {
    let repo = get(gh, &issue.repo_path())?;
    let default = repo["default_branch"]
        .as_str()
        .ok_or_else(|| Error::Json("default_branch".to_owned()))?;
    let tip = get(
        gh,
        &format!(
            "repos/{}/{}/git/ref/heads/{default}",
            issue.owner, issue.repo
        ),
    )?;
    let sha = tip
        .pointer("/object/sha")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| Error::Json("sha".to_owned()))?;
    let path = format!("repos/{}/{}/git/refs", issue.owner, issue.repo);
    let body = serde_json::json!({"ref": format!("refs/heads/{}", issue.branch()), "sha": sha});
    match send(gh, "POST", &path, &body) {
        Ok(_) => Ok(()),
        Err(error) if is_status(&error, 422) => Err(Error::Refused(format!(
            "the task is already taken: the branch {} exists",
            issue.branch()
        ))),
        Err(error) => Err(error),
    }
}

fn mark(gh: &dyn Gh, project: &ProjectLink, items: &[Item], item: &Item) -> Result<(), Error> {
    let field = status_field(gh, project)?;
    set_status(gh, project, &field, item.item_id, &field.in_progress)?;
    let mut parent = item.parent.clone();
    while let Some(link) = parent.take() {
        let Some(next) = items.iter().find(|item| item.link == link) else {
            break;
        };
        if next.open && next.status.as_deref() != Some("In Progress") {
            set_status(gh, project, &field, next.item_id, &field.in_progress)?;
        }
        parent.clone_from(&next.parent);
    }
    Ok(())
}

fn delete_branch(gh: &dyn Gh, issue: &IssueLink) {
    let path = format!(
        "repos/{}/{}/git/refs/heads/{}",
        issue.owner,
        issue.repo,
        issue.branch()
    );
    match gh.run(&["api", "-X", "DELETE", &path], None) {
        Ok(_) | Err(_) => {}
    }
}
