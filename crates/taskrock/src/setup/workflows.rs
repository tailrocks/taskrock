//! Step 9 of `taskrock setup`: the six default workflows of each project and
//! the group-by of each board.

use serde::Deserialize;
use serde_json::json;

use super::{Action, Change, Run};
use crate::config::{self, Workflows};
use crate::error::Error;
use crate::github;
use crate::projects::OrgProject;

#[cfg(test)]
mod tests;

/// The workflow names, as GitHub shows them, in the order of the config keys.
pub(super) const NAMES: [&str; 6] = [
    "Auto-add sub-issues to project",
    "Item added to project",
    "Item closed",
    "Pull request linked to issue",
    "Pull request merged",
    "Auto-close issue",
];

const READ: &str = "query($p:ID!){node(id:$p){... on ProjectV2{workflows(first:20){nodes{id name enabled}} views(first:20){nodes{name layout groupByFields(first:5){nodes{... on ProjectV2FieldCommon{name}}}}}}}}";
const DELETE: &str =
    "mutation($w:ID!){deleteProjectV2Workflow(input:{workflowId:$w}){deletedWorkflowId}}";

/// A workflow of a project.
#[derive(Debug, Deserialize)]
struct Workflow {
    id: String,
    name: String,
    enabled: bool,
}

/// A view of a project and the names of the fields it groups by.
#[derive(Debug, Deserialize)]
struct View {
    name: String,
    layout: String,
    #[serde(rename = "groupByFields", default)]
    group_by: Nodes<Field>,
}

#[derive(Debug, Deserialize)]
struct Field {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Nodes<T> {
    nodes: Vec<T>,
}

impl<T> Default for Nodes<T> {
    fn default() -> Self {
        Self { nodes: Vec::new() }
    }
}

/// What a project has now.
struct Present {
    workflows: Vec<Workflow>,
    views: Vec<View>,
}

/// Keep each workflow set to `true`, delete each one set to `false` only with
/// `--yes`, and report each board that does not group by Status. A project
/// that is `None` does not exist yet, which happens only in a dry run: it has
/// the six workflows on and no view to check (fact F3).
pub(super) fn workflows(
    run: &mut Run<'_>,
    list: &[(config::Project, Option<OrgProject>)],
) -> Result<(), Error> {
    for (wanted, project) in list {
        let present = match project {
            Some(project) => read(run, project)?,
            None => Present {
                workflows: NAMES
                    .iter()
                    .map(|name| Workflow {
                        id: String::new(),
                        name: (*name).to_owned(),
                        enabled: true,
                    })
                    .collect(),
                views: Vec::new(),
            },
        };
        for (name, on) in keys(wanted.workflows) {
            let found = present.workflows.iter().find(|w| w.name == name);
            workflow(run, &wanted.title, name, on, found)?;
        }
        boards(run, &present.views);
    }
    Ok(())
}

/// The workflow names with the value of their config key.
fn keys(set: Workflows) -> [(&'static str, bool); 6] {
    let on = [
        set.auto_add_sub_issues,
        set.item_added,
        set.item_closed,
        set.pr_linked,
        set.pr_merged,
        set.auto_close_issue,
    ];
    std::array::from_fn(|index| (NAMES[index], on[index]))
}

fn read(run: &Run<'_>, project: &OrgProject) -> Result<Present, Error> {
    let data = github::graphql(run.gh, READ, &json!({"p": project.id}))?;
    let node = data
        .get("node")
        .ok_or_else(|| Error::Json("the answer has no project".to_owned()))?;
    let nodes = |key: &str| {
        node.pointer(&format!("/{key}/nodes"))
            .cloned()
            .ok_or_else(|| Error::Json(format!("the answer has no project {key}")))
    };
    let parse = |error: serde_json::Error| Error::Json(error.to_string());
    Ok(Present {
        workflows: serde_json::from_value(nodes("workflows")?).map_err(parse)?,
        views: serde_json::from_value(nodes("views")?).map_err(parse)?,
    })
}

/// Apply one config key to the workflow it names.
fn workflow(
    run: &mut Run<'_>,
    title: &str,
    name: &str,
    on: bool,
    found: Option<&Workflow>,
) -> Result<(), Error> {
    let said = |action, detail: &str| Change {
        action,
        kind: "workflow",
        name: format!("{name}{detail}"),
    };
    match (on, found) {
        (true, Some(found)) if found.enabled => run.say(said(Action::Exists, "")),
        (true, _) => {
            return Err(Error::Project(format!(
                "project {title}: the workflow {name} is off and the API cannot turn it on"
            )));
        }
        (false, None) => run.say(said(Action::Exists, " (deleted)")),
        (false, Some(_)) if !run.yes => run.note(format!(
            "workflow {name} is set to false. Run again with --yes to delete it for good"
        )),
        (false, Some(found)) => {
            run.say(said(Action::Delete, ""));
            if !run.dry_run {
                github::graphql(run.gh, DELETE, &json!({"w": found.id}))?;
            }
        }
    }
    Ok(())
}

/// Say each board that groups by something else than Status. No group-by is
/// fine: a board made through the API groups by Status (fact F2).
fn boards(run: &mut Run<'_>, views: &[View]) {
    for view in views.iter().filter(|view| view.layout == "BOARD_LAYOUT") {
        let names: Vec<&str> = view
            .group_by
            .nodes
            .iter()
            .filter_map(|field| field.name.as_deref())
            .collect();
        if names.is_empty() || names == ["Status"] {
            continue;
        }
        run.note(format!(
            "view {} groups by {}, not Status. The API cannot change it",
            view.name,
            names.join(", ")
        ));
    }
}
