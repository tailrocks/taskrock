//! The creation of the missing issues.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use crate::apply::Created;
use crate::apply::marker::marker;
use crate::apply::split_repo;
use crate::draft::{Draft, DraftIssue};
use crate::error::Error;
use crate::fields::ROUTING;
use crate::gh::Gh;
use crate::github::{is_status, send};
use crate::link::IssueLink;

#[cfg(test)]
mod tests;

/// The issues in creation order: a parent comes before its children,
/// file-name order otherwise.
#[must_use]
pub(super) fn creation_order(draft: &Draft) -> Vec<&DraftIssue> {
    let mut done = BTreeSet::new();
    let mut ordered = Vec::with_capacity(draft.issues.len());
    while ordered.len() < draft.issues.len() {
        let mut moved = false;
        for issue in &draft.issues {
            if done.contains(issue.key.as_str()) {
                continue;
            }
            let ready = issue
                .parent
                .as_deref()
                .is_none_or(|parent| done.contains(parent));
            if ready {
                done.insert(issue.key.as_str());
                ordered.push(issue);
                moved = true;
            }
        }
        if !moved {
            for issue in &draft.issues {
                if done.insert(issue.key.as_str()) {
                    ordered.push(issue);
                }
            }
        }
    }
    ordered
}

/// Creates `issue` on GitHub.
///
/// # Errors
///
/// Returns [`Error::Apply`] with the message GitHub gave when it refuses the
/// creation with HTTP 422, [`Error::Project`] when the organization has no
/// issue field the draft uses, and [`Error::Gh`] or [`Error::Json`] when
/// GitHub fails or answers badly.
pub(super) fn create_one(
    gh: &dyn Gh,
    draft: &Draft,
    issue: &DraftIssue,
    found: &BTreeMap<String, Created>,
    fields: &BTreeMap<String, u64>,
    milestones: &BTreeMap<(String, String), u64>,
) -> Result<Created, Error> {
    let (owner, name) = split_repo(&issue.repo)?;
    let path = format!("repos/{owner}/{name}/issues");
    let body = issue_body(draft, issue, found, fields, milestones)?;
    match send(gh, "POST", &path, &body) {
        Ok(answer) => {
            let number = answer["number"]
                .as_u64()
                .ok_or_else(|| Error::Json("number".to_owned()))?;
            let id = answer["id"]
                .as_u64()
                .ok_or_else(|| Error::Json("id".to_owned()))?;
            Ok(Created {
                link: IssueLink {
                    owner,
                    repo: name,
                    number,
                },
                id,
            })
        }
        Err(error) if is_status(&error, 422) => Err(Error::Apply(github_message(&error))),
        Err(error) => Err(error),
    }
}

/// The `POST` body that creates `issue`.
fn issue_body(
    draft: &Draft,
    issue: &DraftIssue,
    found: &BTreeMap<String, Created>,
    fields: &BTreeMap<String, u64>,
    milestones: &BTreeMap<(String, String), u64>,
) -> Result<Value, Error> {
    let mut body = Map::new();
    body.insert("title".to_owned(), Value::String(issue.title.clone()));
    body.insert(
        "body".to_owned(),
        Value::String(format!(
            "{}\n\n{}",
            issue.body,
            marker(&draft.plan.id, &issue.key)
        )),
    );
    body.insert("type".to_owned(), Value::String(issue.kind.clone()));
    if let Some(title) = &issue.milestone
        && let Some(number) = milestones.get(&(issue.repo.clone(), title.clone()))
    {
        body.insert("milestone".to_owned(), Value::Number((*number).into()));
    }
    if let Some(parent) = &issue.parent {
        let id = found
            .get(parent)
            .map(|known| known.id)
            .ok_or_else(|| Error::Json("parent".to_owned()))?;
        body.insert("parent_issue_id".to_owned(), Value::Number(id.into()));
    }
    let values = field_values(issue, fields)?;
    if !values.is_empty() {
        body.insert("issue_field_values".to_owned(), Value::Array(values));
    }
    Ok(Value::Object(body))
}

/// One `{field_id, value}` entry per routing key the issue has.
fn field_values(issue: &DraftIssue, fields: &BTreeMap<String, u64>) -> Result<Vec<Value>, Error> {
    let mut values = Vec::new();
    for field in ROUTING {
        let Some(value) = issue.routing.get(field).filter(|got| !got.is_empty()) else {
            continue;
        };
        let Some(id) = fields.get(field) else {
            return Err(Error::Project(format!(
                "the organization has no issue field {field}"
            )));
        };
        let mut entry = Map::new();
        entry.insert("field_id".to_owned(), Value::Number((*id).into()));
        entry.insert("value".to_owned(), Value::String(value.to_owned()));
        values.push(Value::Object(entry));
    }
    Ok(values)
}

/// The message GitHub gave for a failed call.
fn github_message(error: &Error) -> String {
    match error {
        Error::Gh { stderr, .. } => stderr.clone(),
        _ => error.to_string(),
    }
}
