//! The rules about links between issues: duplicates, kinds, parents, blockers, cycles, milestones and family size.

use std::collections::{BTreeMap, BTreeSet};

use super::Finding;
use super::{at, by_key, can_be_a_leaf, can_have_children, has_children, keys};
use crate::config::Config;
use crate::draft::{Draft, DraftIssue, links};

pub(super) fn duplicates(draft: &Draft, out: &mut Vec<Finding>) {
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

pub(super) fn kinds(draft: &Draft, config: &Config, out: &mut Vec<Finding>) {
    let names = config.type_names();
    for issue in &draft.issues {
        if !names.contains(&issue.kind.as_str()) {
            let message = format!("type must be one of {}", names.join(", "));
            out.push(at(issue, message));
        }
    }
}

pub(super) fn parents(draft: &Draft, out: &mut Vec<Finding>) {
    let known = keys(draft);
    for issue in &draft.issues {
        let Some(parent) = &issue.parent else {
            if issue.kind != "Initiative" {
                let message = format!(
                    "{} has no parent. Only an Initiative has none: set parent to a key of this draft or to the link of the parent issue",
                    issue.key
                );
                out.push(at(issue, message));
            }
            continue;
        };
        if !known.contains(parent.as_str()) && links::parse(parent).is_none() {
            out.push(at(issue, not_found("parent", parent, &issue.repo)));
        }
    }
}

pub(super) fn blockers(draft: &Draft, out: &mut Vec<Finding>) {
    let known = keys(draft);
    for issue in &draft.issues {
        for blocker in &issue.blocked_by {
            if !known.contains(blocker.as_str()) && links::parse(blocker).is_none() {
                out.push(at(issue, not_found("blocker", blocker, &issue.repo)));
            }
        }
    }
}

/// The message for a `parent` or `blocked_by` value that is neither a key of
/// the draft nor an issue link. `repo` is the repository of the issue.
fn not_found(word: &str, value: &str, repo: &str) -> String {
    let digits = value.strip_prefix('#').unwrap_or(value);
    if !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()) {
        format!(
            "{word} {value} is an issue number, not a key: write the issue link https://github.com/{repo}/issues/{digits}"
        )
    } else if links::is_link(value) || value.starts_with("http://") {
        format!(
            "{word} {value} is not an issue link like https://github.com/<owner>/<repo>/issues/<number>"
        )
    } else {
        format!("{word} {value} is not a key in this draft")
    }
}

pub(super) fn link_cycles(draft: &Draft, out: &mut Vec<Finding>) {
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

pub(super) fn known_links<'link>(
    links: &'link [String],
    known: &BTreeSet<&str>,
) -> Vec<&'link str> {
    links
        .iter()
        .map(String::as_str)
        .filter(|link| known.contains(link))
        .collect()
}

pub(super) fn report_cycles(
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

pub(super) fn find_return(
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

pub(super) fn milestones(draft: &Draft, out: &mut Vec<Finding>) {
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

pub(super) fn family(draft: &Draft, out: &mut Vec<Finding>) {
    let children = has_children(draft);
    for issue in &draft.issues {
        let count = children.get(issue.key.as_str()).map_or(0, Vec::len);
        if count > 0 && !can_have_children(&issue.kind) {
            let message = format!(
                "{} has children, so its type cannot be {}",
                issue.key, issue.kind
            );
            out.push(at(issue, message));
        }
        if count == 0 && !can_be_a_leaf(&issue.kind) {
            let message = format!(
                "{} has no children, so its type must be Task, Bug or Feature",
                issue.key
            );
            out.push(at(issue, message));
        }
    }
}

pub(super) fn family_size(draft: &Draft, out: &mut Vec<Finding>) {
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

pub(super) fn depth(
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
