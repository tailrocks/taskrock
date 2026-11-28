//! The tasks that can start now: `taskrock ready`.

use std::collections::{BTreeMap, BTreeSet};

use crate::config::{Config, ProjectKind};
use crate::error::Error;
use crate::gh::Gh;
use crate::github::items::{Item, project_items, project_of};
use crate::link::{IssueLink, Link};
use crate::projects::kind_of;
use crate::stack::Stack;
use output::{json_of, text_of};

mod body;
#[cfg(test)]
mod loose_tests;
mod output;
#[cfg(test)]
mod problems_tests;
#[cfg(test)]
mod stacked_tests;
mod stacks;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod triage_tests;

/// The scope sorted into groups, with the problems.
#[derive(Debug)]
pub struct Report<'a> {
    /// The candidates to build first, most blocking first.
    pub ready: Vec<&'a Item>,
    /// The open issues with `Agent` set to `Owner`.
    pub waiting_on_owner: Vec<&'a Item>,
    /// The open issues with Status Triage. They are not candidates and not
    /// problems: a manager reads them to see what was reported.
    pub triage: Vec<&'a Item>,
    /// The open issues of the link's milestone that have no parent. They are
    /// not candidates and not problems: a manager reads them to see what was
    /// filed outside its tree.
    pub loose: Vec<&'a Item>,
    /// How many candidates are in progress.
    pub in_progress: usize,
    /// How many candidates have an open blocker.
    pub blocked: usize,
    /// One line per problem.
    pub problems: Vec<String>,
    /// The ready items that start from the branch of an open blocker.
    pub stacked: BTreeMap<IssueLink, Stack>,
}

/// True when the issue can be built: open, no children, a `Task`, a `Bug` or
/// a `Feature`, and not in Triage.
#[must_use]
pub fn is_candidate(item: &Item) -> bool {
    item.open && item.children == 0 && is_task_or_bug(item) && !is_triage(item)
}

/// True when the issue is open and its Status is Triage.
#[must_use]
pub fn is_triage(item: &Item) -> bool {
    item.open && item.status.as_deref() == Some("Triage")
}

/// The items a link covers.
///
/// A project link covers every item. An issue link covers the issue itself
/// and every issue below it: its parent chain reaches the issue.
#[must_use]
pub fn scope<'a>(items: &'a [Item], link: &Link) -> Vec<&'a Item> {
    match link {
        Link::Project(_) => items.iter().collect(),
        Link::Issue(target) => {
            let by_link: BTreeMap<&IssueLink, &Item> =
                items.iter().map(|item| (&item.link, item)).collect();
            items
                .iter()
                .filter(|item| item.link == *target || below(&by_link, item, target))
                .collect()
        }
    }
}

/// The open issues of the milestone of an issue link that have no parent.
///
/// A project link, or an issue link whose issue has no milestone, finds none.
/// A loose item is a `Task`, `Bug`, `Feature` or `Epic` and is not the link
/// itself.
#[must_use]
pub fn loose<'a>(items: &'a [Item], link: &Link) -> Vec<&'a Item> {
    let Link::Issue(target) = link else {
        return Vec::new();
    };
    let milestone = items
        .iter()
        .find(|item| item.link == *target)
        .and_then(|item| item.milestone.as_deref());
    let Some(milestone) = milestone else {
        return Vec::new();
    };
    items
        .iter()
        .filter(|item| {
            item.open
                && item.link != *target
                && item.parent.is_none()
                && item.milestone.as_deref() == Some(milestone)
                && matches!(
                    item.kind.as_deref(),
                    Some("Task" | "Bug" | "Feature" | "Epic")
                )
        })
        .collect()
}

/// The issues that have a claim branch.
///
/// For each repository in the scope, reads
/// `{repo_path}/git/matching-refs/heads/feat/issue-` and keeps every `ref`
/// that ends in digits right after `issue-`.
///
/// # Errors
///
/// Returns [`Error::Gh`] when `gh` fails, [`Error::Json`] when the answer is
/// not JSON.
pub fn claimed(gh: &dyn Gh, scope: &[&Item]) -> Result<BTreeSet<IssueLink>, Error> {
    let mut repos = BTreeSet::new();
    for item in scope {
        repos.insert((item.link.owner.clone(), item.link.repo.clone()));
    }
    let mut out = BTreeSet::new();
    for (owner, repo) in repos {
        let path = format!("repos/{owner}/{repo}/git/matching-refs/heads/feat/issue-");
        for value in crate::github::get_all(gh, &path)? {
            if let Some(link) = claim_of(&value, &owner, &repo) {
                out.insert(link);
            }
        }
    }
    Ok(out)
}

/// Sorts the scope into the groups.
///
/// A candidate that would be ready but whose body breaks the build sections
/// of its type is not ready. Ready tasks that name the same file are warned
/// about.
#[must_use]
pub fn report<'a>(
    scope: &[&'a Item],
    claimed: &BTreeSet<IssueLink>,
    bodies: &BTreeMap<IssueLink, String>,
    config: &Config,
    stacks: &BTreeMap<IssueLink, Stack>,
) -> Report<'a> {
    let mut report = Report {
        ready: Vec::new(),
        waiting_on_owner: Vec::new(),
        triage: Vec::new(),
        loose: Vec::new(),
        in_progress: 0,
        blocked: 0,
        problems: Vec::new(),
        stacked: BTreeMap::new(),
    };
    for item in scope {
        if is_triage(item) {
            report.triage.push(item);
            continue;
        }
        push_problems(&mut report.problems, item);
        if item.open && item.routing.is_owner() {
            report.waiting_on_owner.push(item);
        } else if is_candidate(item) {
            sort_candidate(&mut report, item, claimed, bodies, config, stacks);
        }
    }
    report.ready.sort_by(|a, b| {
        b.blocking
            .cmp(&a.blocking)
            .then_with(|| a.link.number.cmp(&b.link.number))
    });
    warn_shared_files(&mut report, bodies);
    let broken_ready = scope.iter().any(|item| {
        is_candidate(item)
            && !is_triage(item)
            && !item.routing.is_owner()
            && is_ready(item, claimed, stacks)
    });
    if report.ready.is_empty() && report.in_progress == 0 && report.blocked > 0 && !broken_ready {
        report
            .problems
            .push("No task is ready and none is in progress. Look for a blocker cycle.".to_owned());
    }
    report
}

/// The whole command. Returns the text to print.
///
/// A project link lists the whole project, unless it is a portfolio project.
/// An issue link finds its project
/// with `project_of`, reads the items, then keeps the issue and below.
///
/// # Errors
///
/// Returns [`Error::Gh`] when `gh` fails, [`Error::GraphQl`] when GitHub answers with `errors`,
/// [`Error::Json`] when the answer is
/// not JSON, [`Error::Link`] when the link is not understood, [`Error::Project`]
/// when a field or the project is missing, [`Error::Refused`] when the link is
/// a portfolio project.
pub fn run(gh: &dyn Gh, link: &str, json: bool, stacked: bool) -> Result<String, Error> {
    let link: Link = link.parse()?;
    let items = match &link {
        Link::Project(project) => {
            if kind_of(gh, project)? == ProjectKind::Portfolio {
                return Err(Error::Refused(format!(
                    "{} is a portfolio project. Give a delivery project or an initiative",
                    project.url()
                )));
            }
            project_items(gh, project)?
        }
        Link::Issue(issue) => {
            let project = project_of(gh, issue)?;
            project_items(gh, &project)?
        }
    };
    let owner = match &link {
        Link::Project(project) => &project.owner,
        Link::Issue(issue) => &issue.owner,
    };
    let config = crate::config::load::load(gh, owner)?;
    let scope = scope(&items, &link);
    let claimed = claimed(gh, &scope)?;
    let bodies = body::bodies(gh, &scope)?;
    let stacks = if stacked {
        stacks::stacks(gh, &items, &scope, &claimed)?
    } else {
        BTreeMap::new()
    };
    let mut report = report(&scope, &claimed, &bodies, &config, &stacks);
    report.loose = loose(&items, &link);
    if json {
        json_of(&report)
    } else {
        Ok(text_of(&report))
    }
}

fn is_task_or_bug(item: &Item) -> bool {
    matches!(item.kind.as_deref(), Some("Task" | "Bug" | "Feature"))
}

fn below(by_link: &BTreeMap<&IssueLink, &Item>, item: &Item, target: &IssueLink) -> bool {
    let mut parent = item.parent.as_ref();
    while let Some(link) = parent {
        if link == target {
            return true;
        }
        parent = by_link.get(link).and_then(|found| found.parent.as_ref());
    }
    false
}

fn claim_of(value: &serde_json::Value, owner: &str, repo: &str) -> Option<IssueLink> {
    let rest = value
        .get("ref")?
        .as_str()?
        .strip_prefix("refs/heads/feat/issue-")?;
    if rest.is_empty() || !rest.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    rest.parse::<u64>().ok().map(|number| IssueLink {
        owner: owner.to_owned(),
        repo: repo.to_owned(),
        number,
    })
}

fn is_ready(
    item: &Item,
    claimed: &BTreeSet<IssueLink>,
    stacks: &BTreeMap<IssueLink, Stack>,
) -> bool {
    (item.open_blockers == 0 || stacks.contains_key(&item.link))
        && item.status.as_deref() == Some("Todo")
        && !claimed.contains(&item.link)
}

fn sort_candidate<'a>(
    report: &mut Report<'a>,
    item: &'a Item,
    claimed: &BTreeSet<IssueLink>,
    bodies: &BTreeMap<IssueLink, String>,
    config: &Config,
    stacks: &BTreeMap<IssueLink, Stack>,
) {
    if is_ready(item, claimed, stacks) {
        if let Some(stack) = stacks.get(&item.link) {
            report.stacked.insert(item.link.clone(), stack.clone());
        }
        let broken = match bodies.get(&item.link) {
            Some(text) => body::build_problems(item, text, config),
            None => Vec::new(),
        };
        if broken.is_empty() {
            report.ready.push(item);
        }
        let kind = item.kind.as_deref().unwrap_or_default();
        for message in broken {
            let url = item.link.url();
            report
                .problems
                .push(format!("{url} breaks the {kind} format: {message}"));
        }
    } else if claimed.contains(&item.link) || item.status.as_deref() == Some("In Progress") {
        report.in_progress += 1;
    } else if item.open_blockers > 0 {
        report.blocked += 1;
    }
}

fn warn_shared_files(report: &mut Report<'_>, bodies: &BTreeMap<IssueLink, String>) {
    let mut named: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for item in &report.ready {
        if let Some(text) = bodies.get(&item.link) {
            for file in body::named_files(text) {
                named.entry(file).or_default().push(item.link.url());
            }
        }
    }
    for (file, urls) in named {
        if urls.len() >= 2 {
            let list = urls.join(", ");
            report.problems.push(format!(
                "{file} is named by {list}. Build them one after another"
            ));
        }
    }
}

fn push_problems(problems: &mut Vec<String>, item: &Item) {
    let url = item.link.url();
    if item.open && matches!(item.kind.as_deref(), Some("Task" | "Bug")) && item.children > 0 {
        problems.push(format!(
            "{url} has children. Only an issue with no children is built."
        ));
    }
    if is_candidate(item) {
        if item.status.is_none() {
            problems.push(format!("{url} has no status. Is it in the project?"));
        }
        if !item.routing.is_owner() {
            for field in item.routing.missing() {
                problems.push(format!("{url} has no {field}"));
            }
        }
    }
}
