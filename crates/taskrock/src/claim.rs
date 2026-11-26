//! Take a task: create its branch and move it to `In Progress`.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::load::load;
use crate::error::Error;
use crate::format;
use crate::gh::Gh;
use crate::github::items::{Item, project_items, project_of, set_status, status_field};
use crate::github::{get, is_status, send};
use crate::link::{IssueLink, ProjectLink};

#[cfg(test)]
mod body_tests;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod release_tests;

/// Take the task at `link`: create `feat/issue-<number>` and move the task
/// and its open parents to `In Progress`.
///
/// Returns two lines: `taken <url>` and `branch feat/issue-<number>`.
///
/// # Errors
///
/// Returns [`Error::Link`] when `link` is not an issue link, [`Error::Project`]
/// when the issue is not an item of its project, [`Error::Refused`] when the
/// issue cannot be taken or its body breaks the build sections of its type,
/// and [`Error::Gh`] or [`Error::Json`] when GitHub fails or answers badly.
pub fn claim(gh: &dyn Gh, link: &str) -> Result<String, Error> {
    let issue = link.parse::<IssueLink>()?;
    let project = project_of(gh, &issue)?;
    let items = project_items(gh, &project)?;
    let item = find(&items, &issue)?.clone();
    refuse(&item)?;
    check_body(gh, &issue, &item)?;
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
    if !matches!(item.kind.as_deref(), Some("Task" | "Bug" | "Feature")) {
        return Err(Error::Refused(
            "only a Task, a Bug or a Feature can be taken".to_owned(),
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

fn check_body(gh: &dyn Gh, issue: &IssueLink, item: &Item) -> Result<(), Error> {
    let data = get(
        gh,
        &format!("{}/issues/{}", issue.repo_path(), issue.number),
    )?;
    let body = data["body"].as_str().unwrap_or_default();
    let config = load(gh, &issue.owner)?;
    let kind = item.kind.as_deref().unwrap_or_default();
    let Some(format) = config.format(kind, false) else {
        return Err(Error::Refused(format!(
            "the config has no issue type {kind}"
        )));
    };
    let problems = format::problems(body, &format.build, &format.checklist);
    if problems.is_empty() {
        return Ok(());
    }
    Err(Error::Refused(format!(
        "the body breaks the {kind} format: {}",
        problems.join("; ")
    )))
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

/// Give the task at `link` back: free its branch and move it and its free
/// parents to `Todo`.
///
/// Returns two lines: `released <url>` and what happened to the branch:
/// `branch deleted`, `branch kept as <name>` or `no branch`.
///
/// # Errors
///
/// Returns [`Error::Link`] when `link` is not an issue link, [`Error::Project`]
/// when the issue is not an item of its project, [`Error::Refused`] when the
/// issue is closed, and [`Error::Gh`] or [`Error::Json`] when GitHub fails or
/// answers badly.
pub fn release(gh: &dyn Gh, link: &str) -> Result<String, Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| Error::Json(error.to_string()))?
        .as_secs();
    release_at(gh, link, now)
}

fn release_at(gh: &dyn Gh, link: &str, now: u64) -> Result<String, Error> {
    let issue = link.parse::<IssueLink>()?;
    let project = project_of(gh, &issue)?;
    let items = project_items(gh, &project)?;
    let item = find(&items, &issue)?.clone();
    if !item.open {
        return Err(Error::Refused("the issue is closed".to_owned()));
    }
    let branch = finish_branch(gh, &issue, now)?;
    if item.status.as_deref() != Some("Todo") {
        let field = status_field(gh, &project)?;
        set_status(gh, &project, &field, item.item_id, &field.todo)?;
    }
    release_parents(gh, &project, &items, &item)?;
    Ok(format!("released {}\n{branch}", issue.url()))
}

fn finish_branch(gh: &dyn Gh, issue: &IssueLink, now: u64) -> Result<String, Error> {
    let path = format!(
        "repos/{}/{}/git/ref/heads/{}",
        issue.owner,
        issue.repo,
        issue.branch()
    );
    if let Err(error) = get(gh, &path) {
        if is_status(&error, 404) {
            return Ok("no branch".to_owned());
        }
        return Err(error);
    }
    let repo = get(gh, &issue.repo_path())?;
    let default = repo["default_branch"]
        .as_str()
        .ok_or_else(|| Error::Json("default_branch".to_owned()))?;
    let compare = get(
        gh,
        &format!(
            "repos/{}/{}/compare/{default}...{}",
            issue.owner,
            issue.repo,
            issue.branch()
        ),
    )?;
    let ahead = compare
        .pointer("/ahead_by")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| Error::Json("ahead_by".to_owned()))?;
    if ahead == 0 {
        let path = format!(
            "repos/{}/{}/git/refs/heads/{}",
            issue.owner,
            issue.repo,
            issue.branch()
        );
        gh.run(&["api", "-X", "DELETE", &path], None)?;
        return Ok("branch deleted".to_owned());
    }
    let name = format!("{}-parked-{now}", issue.branch());
    let path = format!(
        "repos/{}/{}/branches/{}/rename",
        issue.owner,
        issue.repo,
        issue.branch()
    );
    send(gh, "POST", &path, &serde_json::json!({"new_name": name}))?;
    Ok(format!("branch kept as {name}"))
}

fn release_parents(
    gh: &dyn Gh,
    project: &ProjectLink,
    items: &[Item],
    item: &Item,
) -> Result<(), Error> {
    let mut view = items.to_vec();
    if let Some(own) = view
        .iter_mut()
        .find(|candidate| candidate.link == item.link)
    {
        own.status = Some("Todo".to_owned());
    }
    let mut reset = Vec::new();
    let mut parent = item.parent.clone();
    while let Some(link) = parent.take() {
        let Some(position) = view.iter().position(|candidate| candidate.link == link) else {
            break;
        };
        if is_busy(&view, position) {
            break;
        }
        if view[position].status.as_deref() == Some("In Progress") {
            reset.push(view[position].item_id);
            view[position].status = Some("Todo".to_owned());
        }
        parent.clone_from(&view[position].parent);
    }
    if reset.is_empty() {
        return Ok(());
    }
    let field = status_field(gh, project)?;
    for item_id in reset {
        set_status(gh, project, &field, item_id, &field.todo)?;
    }
    Ok(())
}

fn is_busy(view: &[Item], position: usize) -> bool {
    let link = view[position].link.clone();
    view.iter().any(|child| {
        child.parent.as_ref() == Some(&link)
            && (!child.open || matches!(child.status.as_deref(), Some("In Progress" | "Done")))
    })
}
