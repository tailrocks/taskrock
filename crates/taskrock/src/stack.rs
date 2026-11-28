//! The branch that a stacked task starts from.

use serde_json::Value;

use crate::error::Error;
use crate::gh::Gh;
use crate::github::get_all;
use crate::github::items::Item;
use crate::link::IssueLink;

#[cfg(test)]
mod tests;

/// How many base branches the chain of a blocker follows.
const CHAIN_LIMIT: usize = 20;

/// The branch that a stacked task starts from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stack {
    /// The open blocker whose branch the task starts from.
    pub blocker: IssueLink,
    /// The branch of that blocker: `feat/issue-<number>`.
    pub base: String,
    /// The link of the open pull request whose head is `base`.
    pub pull_request: String,
}

/// An open blocker that can be a base: its pull request and the branch of
/// that pull request.
struct Candidate {
    link: IssueLink,
    pull_request: String,
    base_ref: String,
}

/// Find the open blocker of `item` that its branch can start from.
///
/// # Errors
///
/// Returns [`Error::Refused`] when the open blockers cannot be a base, and
/// [`Error::Gh`] or [`Error::Json`] when GitHub fails or answers badly.
pub fn base(gh: &dyn Gh, items: &[Item], item: &Item) -> Result<Stack, Error> {
    let blockers = open_blockers(gh, item)?;
    if blockers.is_empty() {
        return Err(refused("cannot stack: the task has no open blocker"));
    }
    let mut candidates = Vec::new();
    for blocker in &blockers {
        candidates.push(check(gh, items, item, blocker)?);
    }
    let mut tops = Vec::new();
    for candidate in &candidates {
        let chain = chain(gh, candidate)?;
        let holds_all = candidates
            .iter()
            .filter(|other| other.link != candidate.link)
            .all(|other| chain.contains(&other.link.number));
        if holds_all {
            tops.push(candidate);
        }
    }
    if let [top] = tops.as_slice() {
        return Ok(Stack {
            blocker: top.link.clone(),
            base: top.link.branch(),
            pull_request: top.pull_request.clone(),
        });
    }
    let urls: Vec<String> = blockers.iter().map(IssueLink::url).collect();
    Err(refused(&format!(
        "cannot stack: the open blockers {} are not one stack. Build it after all but one of them close",
        urls.join(", ")
    )))
}

fn refused(message: &str) -> Error {
    Error::Refused(message.to_owned())
}

fn open_blockers(gh: &dyn Gh, item: &Item) -> Result<Vec<IssueLink>, Error> {
    let path = format!(
        "{}/dependencies/blocked_by?per_page=100",
        item.link.api_path()
    );
    let mut open = Vec::new();
    for entry in get_all(gh, &path)? {
        if entry["state"].as_str() != Some("open") {
            continue;
        }
        let url = entry["html_url"]
            .as_str()
            .ok_or_else(|| Error::Json("html_url".to_owned()))?;
        open.push(url.parse::<IssueLink>()?);
    }
    Ok(open)
}

fn check(
    gh: &dyn Gh,
    items: &[Item],
    item: &Item,
    blocker: &IssueLink,
) -> Result<Candidate, Error> {
    let url = blocker.url();
    if blocker.owner != item.link.owner || blocker.repo != item.link.repo {
        return Err(Error::Refused(format!(
            "cannot stack on {url}: it is in another repository"
        )));
    }
    let status = items
        .iter()
        .find(|candidate| candidate.link == *blocker)
        .and_then(|candidate| candidate.status.as_deref());
    if status != Some("In Progress") {
        return Err(Error::Refused(format!(
            "cannot stack on {url}: its status is {}, not In Progress",
            status.unwrap_or("none")
        )));
    }
    match open_pull_request(gh, &blocker.owner, &blocker.repo, &blocker.branch())? {
        Some((pull_request, base_ref)) => Ok(Candidate {
            link: blocker.clone(),
            pull_request,
            base_ref,
        }),
        None => Err(Error::Refused(format!(
            "cannot stack on {url}: {} has no open pull request",
            blocker.branch()
        ))),
    }
}

/// The issue numbers below a blocker: follow the base branches of the pull
/// requests while they are `feat/issue-<digits>`.
fn chain(gh: &dyn Gh, candidate: &Candidate) -> Result<Vec<u64>, Error> {
    let mut numbers = Vec::new();
    let mut branch = candidate.base_ref.clone();
    for _ in 0..CHAIN_LIMIT {
        let Some(number) = issue_number(&branch) else {
            break;
        };
        if numbers.contains(&number) {
            break;
        }
        numbers.push(number);
        let link = &candidate.link;
        match open_pull_request(gh, &link.owner, &link.repo, &branch)? {
            Some((_, next)) => branch = next,
            None => break,
        }
    }
    Ok(numbers)
}

pub(crate) fn issue_number(branch: &str) -> Option<u64> {
    let digits = branch.strip_prefix("feat/issue-")?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// The open pull request whose head is `branch`: its link and its base branch.
fn open_pull_request(
    gh: &dyn Gh,
    owner: &str,
    repo: &str,
    branch: &str,
) -> Result<Option<(String, String)>, Error> {
    let path = format!("repos/{owner}/{repo}/pulls?head={owner}:{branch}&state=open&per_page=100");
    let pulls = get_all(gh, &path)?;
    let Some(first) = pulls.first() else {
        return Ok(None);
    };
    let url = text(first, "/html_url")?;
    let base_ref = text(first, "/base/ref")?;
    Ok(Some((url, base_ref)))
}

fn text(value: &Value, pointer: &str) -> Result<String, Error> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| Error::Json("pull_request".to_owned()))
}
