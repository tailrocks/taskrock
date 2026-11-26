//! The Status of the new issues: Triage for a reported issue, Todo for the
//! rest. An issue that already has a Status keeps it, unless this run created
//! it, or it is a Bug at Todo with no Agent.

use std::collections::BTreeMap;

use crate::apply::{Created, Options};
use crate::draft::{Draft, DraftIssue};
use crate::error::Error;
use crate::gh::Gh;
use crate::github::items::{Item, StatusField, project_items, set_status, status_field};

#[cfg(test)]
mod tests;

const TRIAGE: &str = "Triage";

/// The issues the draft reports: each Bug, and each Feature with no children
/// in the draft.
fn reported(draft: &Draft) -> Vec<&DraftIssue> {
    draft
        .issues
        .iter()
        .filter(|issue| match issue.kind.as_str() {
            "Bug" => true,
            "Feature" => !draft
                .issues
                .iter()
                .any(|other| other.parent.as_deref() == Some(issue.key.as_str())),
            _ => false,
        })
        .collect()
}

/// Fails before anything is created when the draft reports an issue and the
/// Status of the project has no option `Triage`.
///
/// # Errors
///
/// Returns [`Error::Project`] when the option is missing, and the errors of
/// [`status_field`].
pub(super) fn require_triage(gh: &dyn Gh, draft: &Draft) -> Result<(), Error> {
    if reported(draft).is_empty() {
        return Ok(());
    }
    triage_option(&status_field(gh, &draft.plan.project)?).map(|_| ())
}

/// Sets the Status of the issues of the draft. Returns the lines to print.
///
/// # Errors
///
/// Returns [`Error::Gh`] when `gh` fails, [`Error::Project`] when the project
/// has no option `Triage`, or the error of [`project_items`].
pub(super) fn set_statuses(
    gh: &dyn Gh,
    draft: &Draft,
    found: &BTreeMap<String, Created>,
    options: &Options,
) -> Result<Vec<String>, Error> {
    let reported_issues = reported(draft);
    let fresh = |issue: &DraftIssue| found.get(&issue.key).is_none_or(|known| known.fresh);
    if options.dry_run {
        return Ok(reported_issues
            .iter()
            .filter(|issue| fresh(issue))
            .map(|issue| format!("would set {} to {TRIAGE}", issue.key))
            .collect());
    }
    let field = status_field(gh, &draft.plan.project)?;
    let triage = if reported_issues.is_empty() {
        ""
    } else {
        triage_option(&field)?
    };
    let mut lines = Vec::new();
    let items = project_items(gh, &draft.plan.project)?;
    // A reported issue goes to Triage when this run created it, it has no
    // Status yet (an earlier run stopped before setting one), or the project
    // set Todo and no one routed it.
    let mut triaged = Vec::new();
    for issue in &draft.issues {
        let Some(item) = item_of(found, issue, &items) else {
            continue;
        };
        if reported_issues.contains(&issue) {
            if fresh(issue) || item.status.is_none() || untriaged(item) {
                triaged.push(issue);
                if item.status.as_deref() != Some(TRIAGE) {
                    change(gh, draft, &field, item, (triage, TRIAGE), &mut lines)?;
                }
            }
        } else if item.status.is_none() {
            change(gh, draft, &field, item, (&field.todo, "Todo"), &mut lines)?;
        }
    }
    if triaged.is_empty() {
        return Ok(lines);
    }
    std::thread::sleep(options.poll);
    let items = project_items(gh, &draft.plan.project)?;
    for issue in triaged {
        if let Some(item) = item_of(found, issue, &items)
            && item.status.as_deref() != Some(TRIAGE)
        {
            change(gh, draft, &field, item, (triage, TRIAGE), &mut lines)?;
        }
    }
    Ok(lines)
}

/// A found Bug at Todo that no one has routed: the project set Todo, not triage.
fn untriaged(item: &Item) -> bool {
    item.open
        && item.status.as_deref() == Some("Todo")
        && item.routing.get("Agent").is_none_or(str::is_empty)
}

/// The ID of the option `Triage`.
fn triage_option(field: &StatusField) -> Result<&str, Error> {
    field.triage.as_deref().ok_or_else(|| {
        Error::Project("the project has no Status option Triage. Run taskrock setup".to_owned())
    })
}

/// The item of `issue`, when the project holds it.
fn item_of<'a>(
    found: &BTreeMap<String, Created>,
    issue: &DraftIssue,
    items: &'a [Item],
) -> Option<&'a Item> {
    let created = found.get(&issue.key)?;
    items.iter().find(|item| item.link == created.link)
}

/// Sets `item` to the option `(id, name)` and records the printed line.
fn change(
    gh: &dyn Gh,
    draft: &Draft,
    field: &StatusField,
    item: &Item,
    (id, name): (&str, &str),
    lines: &mut Vec<String>,
) -> Result<(), Error> {
    set_status(gh, &draft.plan.project, field, item.item_id, id)?;
    lines.push(format!("status: #{} {name}", item.link.number));
    Ok(())
}
