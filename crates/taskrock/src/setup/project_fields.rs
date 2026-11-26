//! Step 5 of `taskrock setup`: the Status options and the issue-field columns
//! of each project.

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Action, Change, Run};
use crate::config::{self, ProjectKind};
use crate::error::Error;
use crate::github;
use crate::projects::OrgProject;

#[cfg(test)]
pub(super) mod tests;

/// The Status options every project has, in this order.
const STATUS: [&str; 4] = ["Triage", "Todo", "In Progress", "Done"];
/// The eight routing fields: the columns of a delivery project.
const ROUTING: [&str; 8] = [
    "Agent",
    "Review agent",
    "Runtime",
    "Review runtime",
    "Model",
    "Review model",
    "Effort",
    "Review effort",
];
/// The columns of a portfolio project.
const DATES: [&str; 2] = ["Start date", "Target date"];

const FIELDS: &str = "query($p:ID!){node(id:$p){... on ProjectV2{fields(first:50){nodes{... on ProjectV2FieldCommon{id name} ... on ProjectV2SingleSelectField{options{id name color description}}}}}}}";
const UPDATE: &str = "mutation($f:ID!,$o:[ProjectV2SingleSelectFieldOptionInput!]!){updateProjectV2Field(input:{fieldId:$f,singleSelectOptions:$o}){projectV2Field{... on ProjectV2SingleSelectField{id}}}}";
const ADD: &str = "mutation($p:ID!,$i:ID!){createProjectV2IssueField(input:{projectId:$p,issueFieldId:$i}){clientMutationId}}";

/// An option of the Status field, as GraphQL returns it.
#[derive(Debug, Deserialize)]
struct StatusOption {
    id: String,
    name: String,
    color: String,
    #[serde(default)]
    description: String,
}

/// What a project has now: its Status field and the names of its fields.
struct Present {
    status_id: String,
    options: Vec<StatusOption>,
    names: Vec<String>,
}

/// Add the missing Status options and the missing columns to each project.
/// A project that is `None` does not exist yet, which happens only in a dry
/// run.
pub(super) fn project_fields(
    run: &mut Run<'_>,
    list: &[(config::Project, Option<OrgProject>)],
) -> Result<(), Error> {
    let mut org_fields = None;
    for (wanted, project) in list {
        let columns = columns(wanted.kind);
        if let Some(project) = project {
            let present = read(run, project)?;
            status(run, wanted, &present)?;
            add_columns(run, &mut org_fields, wanted, project, &present, columns)?;
        } else {
            run.say(create("status", wanted, ""));
            for column in columns {
                run.say(create("column", wanted, &format!(" {column}")));
            }
        }
    }
    Ok(())
}

fn columns(kind: ProjectKind) -> &'static [&'static str] {
    match kind {
        ProjectKind::Delivery => &ROUTING,
        ProjectKind::Portfolio => &DATES,
    }
}

/// The fields of a project, in one GraphQL call.
fn read(run: &Run<'_>, project: &OrgProject) -> Result<Present, Error> {
    let data = github::graphql(run.gh, FIELDS, &json!({"p": project.id}))?;
    let nodes = data
        .pointer("/node/fields/nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Json("the answer has no project fields".to_owned()))?;
    let name_of = |node: &Value| node.get("name").and_then(Value::as_str).map(str::to_owned);
    let names = nodes.iter().filter_map(name_of).collect();
    let status = nodes
        .iter()
        .find(|node| name_of(node).as_deref() == Some("Status"))
        .ok_or_else(|| Error::Project(format!("project {} has no Status field", project.title)))?;
    let status_id = status
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Json("the Status field has no id".to_owned()))?;
    let options = status.get("options").cloned().unwrap_or(Value::Null);
    let options =
        serde_json::from_value(options).map_err(|error| Error::Json(error.to_string()))?;
    Ok(Present {
        status_id: status_id.to_owned(),
        options,
        names,
    })
}

/// Add the missing Status options, after the existing ones, with the IDs of
/// the existing ones (fact F10).
fn status(run: &mut Run<'_>, wanted: &config::Project, present: &Present) -> Result<(), Error> {
    let missing: Vec<&str> = STATUS
        .into_iter()
        .filter(|name| !present.options.iter().any(|option| option.name == *name))
        .collect();
    if missing.is_empty() {
        run.say(Change {
            action: Action::Exists,
            kind: "status",
            name: wanted.title.clone(),
        });
        return Ok(());
    }
    run.say(Change {
        action: Action::Update,
        kind: "status",
        name: format!("{} (add {})", wanted.title, missing.join(", ")),
    });
    if run.dry_run {
        return Ok(());
    }
    let mut options: Vec<Value> = present
        .options
        .iter()
        .map(|option| {
            json!({"id": option.id, "name": option.name, "color": option.color, "description": option.description})
        })
        .collect();
    options.extend(missing.into_iter().map(new_option));
    let variables = json!({"f": present.status_id, "o": options});
    github::graphql(run.gh, UPDATE, &variables)?;
    Ok(())
}

fn new_option(name: &str) -> Value {
    let description = match name {
        "Triage" => "A reported issue that waits for triage",
        _ => "",
    };
    json!({"name": name, "color": "GRAY", "description": description})
}

/// Add each wanted column that the project lacks.
fn add_columns(
    run: &mut Run<'_>,
    org_fields: &mut Option<Vec<(String, String)>>,
    wanted: &config::Project,
    project: &OrgProject,
    present: &Present,
    columns: &[&str],
) -> Result<(), Error> {
    for column in columns {
        let line = |action| Change {
            action,
            kind: "column",
            name: format!("{} {column}", wanted.title),
        };
        if present.names.iter().any(|name| name == column) {
            run.say(line(Action::Exists));
            continue;
        }
        run.say(line(Action::Create));
        if run.dry_run {
            continue;
        }
        if org_fields.is_none() {
            *org_fields = Some(read_org_fields(run)?);
        }
        let node_id = org_fields
            .iter()
            .flatten()
            .find(|(name, _)| name == column)
            .map(|(_, node_id)| node_id)
            .ok_or_else(|| {
                Error::Project(format!("the organization has no issue field {column}"))
            })?;
        let variables = json!({"p": project.id, "i": node_id});
        github::graphql(run.gh, ADD, &variables)?;
    }
    Ok(())
}

/// The name and `node_id` of each issue field of the organization.
fn read_org_fields(run: &Run<'_>) -> Result<Vec<(String, String)>, Error> {
    let answer = github::get(run.gh, &format!("orgs/{}/issue-fields", run.owner))?;
    let text = |field: &Value, key: &str| field.get(key).and_then(Value::as_str).map(str::to_owned);
    Ok(answer
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|field| Some((text(field, "name")?, text(field, "node_id")?)))
        .collect())
}

fn create(kind: &'static str, wanted: &config::Project, detail: &str) -> Change {
    Change {
        action: Action::Create,
        kind,
        name: format!("{}{detail}", wanted.title),
    }
}
