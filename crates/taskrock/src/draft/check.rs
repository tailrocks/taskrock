//! Check a draft against the rules.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::{Draft, DraftIssue};
use crate::error::Error;
use crate::fields::{ROUTING, draft_key};

#[cfg(test)]
mod tests;

/// One broken rule, with the file that breaks it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub file: String,
    pub message: String,
}

/// Every broken rule, sorted by file name, then by message.
#[must_use]
pub fn check(draft: &Draft) -> Vec<Finding> {
    let mut findings = Vec::new();
    duplicates(draft, &mut findings);
    kinds(draft, &mut findings);
    parents(draft, &mut findings);
    blockers(draft, &mut findings);
    link_cycles(draft, &mut findings);
    milestones(draft, &mut findings);
    family(draft, &mut findings);
    routing(draft, &mut findings);
    sections(draft, &mut findings);
    checkboxes(draft, &mut findings);
    bodies(draft, &mut findings);
    titles(draft, &mut findings);
    family_size(draft, &mut findings);
    findings.sort_by(|a, b| (&a.file, &a.message).cmp(&(&b.file, &b.message)));
    findings
}

/// Read the folder, check it, return the text to print.
///
/// # Errors
///
/// Returns [`Error::Draft`] or [`Error::Check`].
pub fn run(folder: &Path) -> Result<String, Error> {
    let draft = super::parse::read(folder)?;
    let findings = check(&draft);
    if findings.is_empty() {
        return Ok(format!("OK {} issues", draft.issues.len()));
    }
    for finding in &findings {
        println!("{}: {}", finding.file, finding.message);
    }
    Err(Error::Check(findings.len()))
}

fn at(issue: &DraftIssue, message: String) -> Finding {
    Finding {
        file: issue.file.clone(),
        message,
    }
}

fn keys(draft: &Draft) -> BTreeSet<&str> {
    draft
        .issues
        .iter()
        .map(|issue| issue.key.as_str())
        .collect()
}

fn by_key(draft: &Draft) -> BTreeMap<&str, &DraftIssue> {
    let mut map = BTreeMap::new();
    for issue in &draft.issues {
        map.entry(issue.key.as_str()).or_insert(issue);
    }
    map
}

fn has_children(draft: &Draft) -> BTreeMap<&str, Vec<&DraftIssue>> {
    let mut map: BTreeMap<&str, Vec<&DraftIssue>> = BTreeMap::new();
    for issue in &draft.issues {
        if let Some(parent) = &issue.parent
            && parent != &issue.key
        {
            map.entry(parent.as_str()).or_default().push(issue);
        }
    }
    map
}

fn is_task(kind: &str) -> bool {
    kind == "Task" || kind == "Bug"
}

fn is_epic(kind: &str) -> bool {
    kind == "Initiative" || kind == "Epic"
}

fn duplicates(draft: &Draft, out: &mut Vec<Finding>) {
    let mut files: BTreeMap<&str, Vec<&DraftIssue>> = BTreeMap::new();
    for issue in &draft.issues {
        files.entry(issue.key.as_str()).or_default().push(issue);
    }
    for (key, mut group) in files.into_iter().filter(|(_, group)| group.len() > 1) {
        group.sort_by(|a, b| a.file.cmp(&b.file));
        let first = group[0].file.clone();
        for issue in &group[1..] {
            out.push(at(issue, format!("key {key} is also used in {first}")));
        }
    }
}

fn kinds(draft: &Draft, out: &mut Vec<Finding>) {
    for issue in &draft.issues {
        if !is_task(&issue.kind) && !is_epic(&issue.kind) {
            let message = "type must be Initiative, Epic, Task or Bug".to_owned();
            out.push(at(issue, message));
        }
    }
}

fn parents(draft: &Draft, out: &mut Vec<Finding>) {
    let known = keys(draft);
    for issue in &draft.issues {
        if let Some(parent) = &issue.parent
            && !known.contains(parent.as_str())
        {
            let message = format!("parent {parent} is not a key in this draft");
            out.push(at(issue, message));
        }
    }
}

fn blockers(draft: &Draft, out: &mut Vec<Finding>) {
    let known = keys(draft);
    for issue in &draft.issues {
        for blocker in &issue.blocked_by {
            if !known.contains(blocker.as_str()) {
                let message = format!("blocker {blocker} is not a key in this draft");
                out.push(at(issue, message));
            }
        }
    }
}

fn link_cycles(draft: &Draft, out: &mut Vec<Finding>) {
    let known = keys(draft);
    let mut blocked: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut parented: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for issue in &draft.issues {
        blocked.insert(issue.key.as_str(), known_links(&issue.blocked_by, &known));
        parented.insert(
            issue.key.as_str(),
            known_links(issue.parent.as_slice(), &known),
        );
    }
    report_cycles(draft, &blocked, "blockers", out);
    report_cycles(draft, &parented, "parents", out);
}

fn known_links<'link>(links: &'link [String], known: &BTreeSet<&str>) -> Vec<&'link str> {
    links
        .iter()
        .map(String::as_str)
        .filter(|link| known.contains(link))
        .collect()
}

fn report_cycles(
    draft: &Draft,
    edges: &BTreeMap<&str, Vec<&str>>,
    word: &str,
    out: &mut Vec<Finding>,
) {
    let index = by_key(draft);
    for issue in &draft.issues {
        let key = issue.key.as_str();
        let mut path = vec![issue.key.clone()];
        let mut visited = BTreeSet::from([issue.key.clone()]);
        if !find_return(key, key, edges, &mut visited, &mut path) {
            continue;
        }
        if path.iter().min().is_none_or(|first| first.as_str() != key) {
            continue;
        }
        out.push(Finding {
            file: index[key].file.clone(),
            message: format!("{word} form a cycle: {}", path.join(" -> ")),
        });
    }
}

fn find_return(
    start: &str,
    current: &str,
    edges: &BTreeMap<&str, Vec<&str>>,
    visited: &mut BTreeSet<String>,
    path: &mut Vec<String>,
) -> bool {
    let mut next: Vec<&str> = edges.get(current).cloned().unwrap_or_default();
    next.sort_unstable();
    for node in next {
        if node == start {
            path.push(start.to_owned());
            return true;
        }
        if visited.insert(node.to_owned()) {
            path.push(node.to_owned());
            if find_return(start, node, edges, visited, path) {
                return true;
            }
            path.pop();
        }
    }
    false
}

fn milestones(draft: &Draft, out: &mut Vec<Finding>) {
    let titles: BTreeSet<&str> = draft
        .plan
        .milestones
        .iter()
        .map(|milestone| milestone.title.as_str())
        .collect();
    for issue in &draft.issues {
        if let Some(milestone) = &issue.milestone
            && !titles.contains(milestone.as_str())
        {
            let message = format!("milestone {milestone} is not in plan.toml");
            out.push(at(issue, message));
        }
    }
}

fn family(draft: &Draft, out: &mut Vec<Finding>) {
    let children = has_children(draft);
    for issue in &draft.issues {
        let count = children.get(issue.key.as_str()).map_or(0, Vec::len);
        if count > 0 && !is_epic(&issue.kind) {
            let message = format!(
                "{} has children, so its type cannot be {}",
                issue.key, issue.kind
            );
            out.push(at(issue, message));
        }
        if count == 0 && !is_task(&issue.kind) {
            let message = format!(
                "{} has no children, so its type must be Task or Bug",
                issue.key
            );
            out.push(at(issue, message));
        }
    }
}

fn routing(draft: &Draft, out: &mut Vec<Finding>) {
    for issue in &draft.issues {
        if is_epic(&issue.kind) && has_routing(issue, 0) {
            let message = "only a Task or Bug has routing fields".to_owned();
            out.push(at(issue, message));
        }
        if !is_task(&issue.kind) {
            continue;
        }
        if issue.routing.get("Agent").is_none_or(str::is_empty) {
            out.push(at(issue, "agent is missing".to_owned()));
        }
        if issue.routing.is_owner() && has_routing(issue, 1) {
            let message = "an Owner task has no other routing field".to_owned();
            out.push(at(issue, message));
        }
        if !issue.routing.is_owner() {
            for field in issue.routing.missing() {
                out.push(at(issue, format!("{} is missing", draft_key(field))));
            }
        }
    }
}

fn has_routing(issue: &DraftIssue, skip: usize) -> bool {
    ROUTING.iter().skip(skip).any(|field| {
        issue
            .routing
            .get(field)
            .is_some_and(|value| !value.is_empty())
    })
}

fn sections(draft: &Draft, out: &mut Vec<Finding>) {
    for issue in &draft.issues {
        let leaf = is_task(&issue.kind) && !issue.routing.is_owner();
        let wanted: &[&str] = if leaf {
            &[
                "## Outcome",
                "## Decisions already made",
                "## Steps",
                "## Done when",
                "## Out of scope",
            ]
        } else {
            &["## Outcome", "## Done when"]
        };
        for header in wanted {
            if !has_section(&issue.body, header) {
                out.push(at(issue, format!("the section {header} is missing")));
            }
        }
    }
}

fn has_section(body: &str, header: &str) -> bool {
    body.lines().any(|line| line.trim() == header)
}

fn checkboxes(draft: &Draft, out: &mut Vec<Finding>) {
    for issue in &draft.issues {
        if has_section(&issue.body, "## Done when") && !has_checkbox(&issue.body) {
            out.push(at(issue, "## Done when has no checkbox".to_owned()));
        }
    }
}

fn has_checkbox(body: &str) -> bool {
    let mut lines = body
        .lines()
        .skip_while(|line| line.trim() != "## Done when");
    lines.next();
    lines
        .take_while(|line| !line.trim().starts_with("## "))
        .any(|line| line.trim_start().starts_with("- [ ]"))
}

fn bodies(draft: &Draft, out: &mut Vec<Finding>) {
    for issue in &draft.issues {
        let count = issue.body.chars().count();
        if count > 60_000 {
            let message = format!("the body has {count} characters. The limit is 60000");
            out.push(at(issue, message));
        }
    }
}

fn titles(draft: &Draft, out: &mut Vec<Finding>) {
    for issue in &draft.issues {
        let count = issue.title.chars().count();
        if count == 0 || count > 256 {
            let message = "the title must have 1 to 256 characters".to_owned();
            out.push(at(issue, message));
        }
    }
}

fn family_size(draft: &Draft, out: &mut Vec<Finding>) {
    let children = has_children(draft);
    let index = by_key(draft);
    let mut depths = BTreeMap::new();
    for issue in &draft.issues {
        let count = children.get(issue.key.as_str()).map_or(0, Vec::len);
        if count > 100 {
            let message = format!("{} has {count} children. The limit is 100", issue.key);
            out.push(at(issue, message));
        }
        let level = depth(issue.key.as_str(), &index, &mut depths);
        if level > 8 {
            let message = format!("{} is {level} levels deep. The limit is 8", issue.key);
            out.push(at(issue, message));
        }
    }
}

fn depth(
    key: &str,
    index: &BTreeMap<&str, &DraftIssue>,
    depths: &mut BTreeMap<String, usize>,
) -> usize {
    if let Some(level) = depths.get(key) {
        return *level;
    }
    let mut level = 1;
    let mut seen = BTreeSet::from([key.to_owned()]);
    let mut current = key.to_owned();
    while let Some(issue) = index.get(current.as_str()) {
        let Some(parent) = &issue.parent else { break };
        if !index.contains_key(parent.as_str()) || !seen.insert(parent.clone()) {
            break;
        }
        level += 1;
        current = parent.clone();
    }
    depths.insert(key.to_owned(), level);
    level
}
