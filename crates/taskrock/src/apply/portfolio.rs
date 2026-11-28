//! Adding the initiatives of a draft to its portfolio project.

use std::collections::BTreeMap;

use crate::apply::project::add;
use crate::apply::{Created, Options, plan_owner};
use crate::config::ProjectKind;
use crate::draft::{Draft, DraftIssue};
use crate::error::Error;
use crate::gh::Gh;
use crate::github::items::project_items;
use crate::link::ProjectLink;
use crate::projects;

#[cfg(test)]
mod tests;

const INITIATIVE: &str = "Initiative";

/// The portfolio project that `plan.toml` names. `None` when it names none.
///
/// # Errors
///
/// Returns [`Error::Project`] when the organization has no portfolio project
/// with that key, and the errors of [`projects::list`].
pub(super) fn find_portfolio(
    gh: &dyn Gh,
    owner: &str,
    draft: &Draft,
) -> Result<Option<ProjectLink>, Error> {
    let Some(key) = &draft.plan.portfolio else {
        return Ok(None);
    };
    projects::list(gh, owner)?
        .into_iter()
        .find(|project| {
            project
                .marker
                .as_ref()
                .is_some_and(|marker| marker.key == *key && marker.kind == ProjectKind::Portfolio)
        })
        .map(|project| ProjectLink {
            owner: owner.to_owned(),
            number: project.number,
        })
        .ok_or_else(|| {
            Error::Project(format!(
                "no portfolio project with key {key}. Run taskrock setup"
            ))
        })
        .map(Some)
}

/// Adds each Initiative of the draft that the portfolio project does not
/// hold yet. Returns the lines to print.
///
/// # Errors
///
/// Returns [`Error::Project`] when the portfolio project is missing,
/// [`Error::Json`] when an Initiative has no issue on GitHub, [`Error::Gh`], [`Error::GraphQl`]
/// when `gh` fails, or the error of [`project_items`].
pub(super) fn add_to_portfolio(
    gh: &dyn Gh,
    draft: &Draft,
    found: &BTreeMap<String, Created>,
    options: &Options,
) -> Result<Vec<String>, Error> {
    let Some(key) = &draft.plan.portfolio else {
        return Ok(Vec::new());
    };
    if options.dry_run {
        return Ok(initiatives(draft)
            .map(|issue| format!("would add {} to portfolio {key}", issue.key))
            .collect());
    }
    let Some(portfolio) = find_portfolio(gh, &plan_owner(draft)?, draft)? else {
        return Ok(Vec::new());
    };
    let items = project_items(gh, &portfolio)?;
    let mut lines = Vec::new();
    for issue in initiatives(draft) {
        let link = found
            .get(&issue.key)
            .map(|known| known.link.clone())
            .ok_or_else(|| Error::Json("portfolio".to_owned()))?;
        if items.iter().all(|item| item.link != link) {
            add(gh, &portfolio, &link, &mut Vec::new())?;
            lines.push(format!("portfolio: added #{}", link.number));
        }
    }
    Ok(lines)
}

/// The Initiatives of the draft.
fn initiatives(draft: &Draft) -> impl Iterator<Item = &DraftIssue> {
    draft.issues.iter().filter(|issue| issue.kind == INITIATIVE)
}
