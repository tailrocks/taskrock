//! Check a draft against the rules.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::{Draft, DraftIssue};
use crate::error::Error;

use self::body::{bodies, checkboxes, routing, sections, titles};
use self::graph::{
    blockers, duplicates, family, family_size, kinds, link_cycles, milestones, parents,
};

#[cfg(test)]
mod tests;

mod body;
mod graph;

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

pub(super) fn at(issue: &DraftIssue, message: String) -> Finding {
    Finding {
        file: issue.file.clone(),
        message,
    }
}

pub(super) fn keys(draft: &Draft) -> BTreeSet<&str> {
    draft
        .issues
        .iter()
        .map(|issue| issue.key.as_str())
        .collect()
}

pub(super) fn by_key(draft: &Draft) -> BTreeMap<&str, &DraftIssue> {
    let mut map = BTreeMap::new();
    for issue in &draft.issues {
        map.entry(issue.key.as_str()).or_insert(issue);
    }
    map
}

pub(super) fn has_children(draft: &Draft) -> BTreeMap<&str, Vec<&DraftIssue>> {
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

pub(super) fn is_task(kind: &str) -> bool {
    kind == "Task" || kind == "Bug"
}

pub(super) fn is_epic(kind: &str) -> bool {
    kind == "Initiative" || kind == "Epic"
}
