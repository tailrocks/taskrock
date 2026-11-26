//! Step 8 of `taskrock setup`: the shared milestones, in each repository of
//! their project.

use serde::Deserialize;
use serde_json::{Map, Value};

use super::{Action, Change, Run};
use crate::config::{self, SharedMilestone};
use crate::error::Error;
use crate::github;
use crate::projects::OrgProject;

#[cfg(test)]
mod tests;

/// A milestone of a repository, as the REST API returns it.
#[derive(Debug, Deserialize)]
struct Present {
    number: u64,
    title: String,
    description: Option<String>,
    due_on: Option<String>,
}

impl Present {
    /// True when the config sets a due date or a description that differs.
    fn differs(&self, wanted: &SharedMilestone) -> bool {
        let other_due = wanted.due.as_ref().is_some_and(|due| {
            self.due_on.as_ref().and_then(|on| on.get(..10)) != Some(due.as_str())
        });
        let other_text = wanted
            .description
            .as_ref()
            .is_some_and(|text| self.description.as_deref().unwrap_or("") != text);
        other_due || other_text
    }
}

/// Create each shared milestone in each repository of its project, and fix
/// its due date and description. A project that is `None` does not exist yet,
/// which happens only in a dry run: nothing is read.
pub(super) fn milestones(
    run: &mut Run<'_>,
    list: &[(config::Project, Option<OrgProject>)],
) -> Result<(), Error> {
    for (wanted, project) in list {
        if wanted.milestones.is_empty() {
            continue;
        }
        for repo in &wanted.repos {
            let present = match project {
                Some(_) => read(run, repo)?,
                None => Vec::new(),
            };
            for milestone in &wanted.milestones {
                fix(run, repo, milestone, &present)?;
            }
        }
    }
    Ok(())
}

/// The milestones of a repository, open and closed.
fn read(run: &Run<'_>, repo: &str) -> Result<Vec<Present>, Error> {
    let path = format!(
        "repos/{}/{repo}/milestones?state=all&per_page=100",
        run.owner
    );
    let answer = github::get(run.gh, &path)?;
    serde_json::from_value(answer).map_err(|error| Error::Json(error.to_string()))
}

/// Create the milestone, fix it, or find that it is right.
fn fix(
    run: &mut Run<'_>,
    repo: &str,
    wanted: &SharedMilestone,
    present: &[Present],
) -> Result<(), Error> {
    let found = present.iter().find(|m| m.title == wanted.title);
    let action = match found {
        None => Action::Create,
        Some(found) if found.differs(wanted) => Action::Update,
        Some(_) => Action::Exists,
    };
    run.say(Change {
        action,
        kind: "milestone",
        name: format!("{repo} {}", wanted.title),
    });
    if run.dry_run {
        return Ok(());
    }
    let mut body = Map::new();
    if let Some(due) = &wanted.due {
        // Noon, not midnight: GitHub stores a POST of `<day>T00:00:00Z` one day
        // earlier, and noon is stored as the same day.
        body.insert("due_on".to_owned(), format!("{due}T12:00:00Z").into());
    }
    if let Some(text) = &wanted.description {
        body.insert("description".to_owned(), text.as_str().into());
    }
    let base = format!("repos/{}/{repo}/milestones", run.owner);
    match (action, found) {
        (Action::Create, _) => {
            body.insert("title".to_owned(), wanted.title.as_str().into());
            github::send(run.gh, "POST", &base, &Value::Object(body))?;
        }
        (Action::Update, Some(found)) => {
            let path = format!("{base}/{}", found.number);
            github::send(run.gh, "PATCH", &path, &Value::Object(body))?;
        }
        _ => {}
    }
    Ok(())
}
