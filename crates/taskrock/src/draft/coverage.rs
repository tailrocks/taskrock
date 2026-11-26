//! `taskrock check --coverage`: a read-only coverage report of a draft.

use std::path::Path;

use super::check::{Finding, at, can_have_children, has_children};
use super::{Draft, DraftIssue};
use crate::config::Config;
use crate::error::Error;
use crate::format;

#[cfg(test)]
mod tests;

/// The most steps a task should have.
const MAX_STEPS: usize = 10;

/// Read the folder, print each note, return the summary line.
///
/// # Errors
///
/// Returns [`Error::Draft`] when the folder cannot be read.
pub fn run(folder: &Path, _config: &Config) -> Result<String, Error> {
    let draft = super::parse::read(folder)?;
    let found = notes(&draft);
    for note in &found {
        println!("{}: {}", note.file, note.message);
    }
    Ok(format!("COVERAGE {} notes", found.len()))
}

/// Every note about a weak spot of the draft, sorted by file name, then by message.
#[must_use]
pub fn notes(draft: &Draft) -> Vec<Finding> {
    let mut found = Vec::new();
    outcomes(draft, &mut found);
    steps(draft, &mut found);
    routing(draft, &mut found);
    found.sort_by(|a, b| (&a.file, &a.message).cmp(&(&b.file, &b.message)));
    found
}

/// The lines of `text` that are list items.
fn items(text: &str) -> usize {
    text.lines().map(str::trim).filter(|l| is_item(l)).count()
}

fn is_item(line: &str) -> bool {
    if line.starts_with("- ") || line.starts_with("* ") {
        return true;
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    digits > 0 && line[digits..].starts_with(". ")
}

fn outcomes(draft: &Draft, out: &mut Vec<Finding>) {
    let children = has_children(draft);
    for issue in &draft.issues {
        if !can_have_children(&issue.kind) {
            continue;
        }
        let Some(kids) = children.get(issue.key.as_str()) else {
            continue;
        };
        let wanted = format::section(&issue.body, "Outcome").map_or(0, |text| items(&text));
        if wanted > kids.len() {
            let message = format!(
                "Outcome has {wanted} items and {} children. Is an item covered by no task?",
                kids.len()
            );
            out.push(at(issue, message));
        }
    }
}

fn steps(draft: &Draft, out: &mut Vec<Finding>) {
    for issue in draft.issues.iter().filter(|issue| issue.kind == "Task") {
        let count = format::section(&issue.body, "Steps").map_or(0, |text| items(&text));
        if count > MAX_STEPS {
            let message = format!("{count} steps. Split a task with more than {MAX_STEPS} steps");
            out.push(at(issue, message));
        }
    }
}

fn routing(draft: &Draft, out: &mut Vec<Finding>) {
    for issue in &draft.issues {
        let works = matches!(issue.kind.as_str(), "Task" | "Bug") && !issue.routing.is_owner();
        if works && is_blank(issue, "Agent") {
            out.push(at(issue, "no builder".to_owned()));
        }
        if works && is_blank(issue, "Review agent") {
            out.push(at(issue, "no reviewer".to_owned()));
        }
        same_model(issue, out);
    }
}

fn is_blank(issue: &DraftIssue, field: &str) -> bool {
    issue.routing.get(field).is_none_or(str::is_empty)
}

fn same_model(issue: &DraftIssue, out: &mut Vec<Finding>) {
    let model = issue.routing.get("Model").filter(|value| !value.is_empty());
    if model.is_some() && model == issue.routing.get("Review model") {
        let message = format!(
            "the reviewer uses the builder's model {}",
            model.unwrap_or_default()
        );
        out.push(at(issue, message));
    }
}
