//! Take a task on the branch of its blocker's open pull request.

use serde_json::json;

use crate::error::Error;
use crate::gh::Gh;
use crate::github::items::{project_items, project_of};
use crate::github::send;
use crate::link::IssueLink;
use crate::stack::{self, Stack};

use super::{check_body, create_branch, delete_branch, find, mark, refuse};

#[cfg(test)]
mod tests;

/// Take the task at `link` like [`super::claim`], but start its branch from
/// the branch of its open blocker, when it has one.
///
/// Returns three lines: `taken <url>`, `branch feat/issue-<number>` and
/// `base <branch>`.
///
/// # Errors
///
/// Returns [`Error::Link`] when `link` is not an issue link, [`Error::Project`]
/// when the issue is not an item of its project, [`Error::Refused`] when the
/// issue cannot be taken or stacked, and [`Error::Gh`] or [`Error::Json`] when
/// GitHub fails or answers badly.
pub fn claim_stacked(gh: &dyn Gh, link: &str) -> Result<String, Error> {
    let issue = link.parse::<IssueLink>()?;
    let project = project_of(gh, &issue)?;
    let items = project_items(gh, &project)?;
    let item = find(&items, &issue)?.clone();
    refuse(&item, true)?;
    if item.open_blockers == 0 {
        check_body(gh, &issue, &item)?;
        let from = create_branch(gh, &issue, None)?;
        if let Err(error) = mark(gh, &project, &items, &item) {
            delete_branch(gh, &issue);
            return Err(error);
        }
        return Ok(taken(&issue, &from));
    }
    let stack = stack::base(gh, &items, &item)?;
    check_body(gh, &issue, &item)?;
    create_branch(gh, &issue, Some(&stack.base))?;
    let comment = match post_comment(gh, &issue, &stack) {
        Ok(id) => id,
        Err(error) => {
            delete_branch(gh, &issue);
            return Err(error);
        }
    };
    if let Err(error) = mark(gh, &project, &items, &item) {
        delete_branch(gh, &issue);
        delete_comment(gh, &issue, comment);
        return Err(error);
    }
    Ok(taken(&issue, &stack.base))
}

fn taken(issue: &IssueLink, base: &str) -> String {
    format!(
        "taken {}\nbranch {}\nbase {base}",
        issue.url(),
        issue.branch()
    )
}

fn post_comment(gh: &dyn Gh, issue: &IssueLink, stack: &Stack) -> Result<u64, Error> {
    let blocker = stack.blocker.url();
    let branch = issue.branch();
    let base = &stack.base;
    let body = format!(
        "Stacked on {blocker}: the branch {branch} starts from {base}, whose pull request is {}. Open the pull request of this task with the base {base}. Before {blocker} merges, the manager retargets it to the default branch.\n\n<!-- taskrock:stacked-on {base} -->",
        stack.pull_request
    );
    let answer = send(
        gh,
        "POST",
        &format!("{}/comments", issue.api_path()),
        &json!({"body": body}),
    )?;
    answer["id"]
        .as_u64()
        .ok_or_else(|| Error::Json("comment id".to_owned()))
}

fn delete_comment(gh: &dyn Gh, issue: &IssueLink, id: u64) {
    let path = format!("repos/{}/{}/issues/comments/{id}", issue.owner, issue.repo);
    match gh.run(&["api", "-X", "DELETE", &path], None) {
        Ok(_) | Err(_) => {}
    }
}
