//! Step 6 of `taskrock setup`: the views of each kind of project.

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Action, Change, Run};
use crate::config::{self, ProjectKind};
use crate::error::Error;
use crate::github;
use crate::projects::OrgProject;

#[cfg(test)]
pub(super) mod tests;

/// The layout of a view: GraphQL `BOARD_LAYOUT`, `TABLE_LAYOUT`, `ROADMAP_LAYOUT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layout {
    Board,
    Table,
    Roadmap,
}

impl Layout {
    fn graphql(self) -> &'static str {
        match self {
            Self::Board => "BOARD_LAYOUT",
            Self::Table => "TABLE_LAYOUT",
            Self::Roadmap => "ROADMAP_LAYOUT",
        }
    }
}

/// One view a project of a kind must have.
struct ViewSpec {
    name: &'static str,
    layout: Layout,
    filter: &'static str,
    columns: &'static [&'static str],
}

const fn spec(
    name: &'static str,
    layout: Layout,
    filter: &'static str,
    columns: &'static [&'static str],
) -> ViewSpec {
    ViewSpec {
        name,
        layout,
        filter,
        columns,
    }
}

const DELIVERY: &[ViewSpec] = &[
    spec(
        "Board",
        Layout::Board,
        "-type:Initiative,Epic",
        &["Title", "Agent", "Model"],
    ),
    spec(
        "Ready",
        Layout::Table,
        "is:open -is:blocked status:Todo -agent:Owner no:sub-issues-progress",
        &[
            "Title",
            "Status",
            "Milestone",
            "Agent",
            "Runtime",
            "Model",
            "Effort",
        ],
    ),
    spec(
        "Triage",
        Layout::Table,
        "status:Triage",
        &["Title", "Status", "Repository", "Milestone"],
    ),
    spec(
        "Needs you",
        Layout::Table,
        "is:open agent:Owner",
        &["Title", "Status", "Milestone", "Parent issue"],
    ),
    spec(
        "Requests",
        Layout::Table,
        "type:Feature",
        &["Title", "Status", "Repository", "Milestone"],
    ),
    spec(
        "All tasks",
        Layout::Table,
        "-type:Initiative,Epic",
        &[
            "Title",
            "Status",
            "Milestone",
            "Agent",
            "Runtime",
            "Model",
            "Effort",
            "Review agent",
            "Review runtime",
            "Review model",
            "Review effort",
        ],
    ),
    spec(
        "Tree",
        Layout::Table,
        "",
        &[
            "Title",
            "Status",
            "Milestone",
            "Sub-issues progress",
            "Agent",
        ],
    ),
];

const PORTFOLIO: &[ViewSpec] = &[
    spec(
        "Initiatives",
        Layout::Table,
        "type:Initiative",
        &[
            "Title",
            "Status",
            "Sub-issues progress",
            "Start date",
            "Target date",
        ],
    ),
    spec(
        "Roadmap",
        Layout::Roadmap,
        "type:Initiative",
        &["Title", "Status", "Sub-issues progress"],
    ),
];

/// The name of the view GitHub puts in a new project.
const DEFAULT_VIEW: &str = "View 1";

const READ: &str = "query($p:ID!){node(id:$p){... on ProjectV2{fields(first:50){nodes{... on ProjectV2FieldCommon{id name}}} views(first:50){nodes{id name layout filter fields(first:30){nodes{... on ProjectV2FieldCommon{id name}}}}}}}}";
const CREATE: &str =
    "mutation($i:CreateProjectV2ViewInput!){createProjectV2View(input:$i){projectV2View{id}}}";
const UPDATE: &str =
    "mutation($i:UpdateProjectV2ViewInput!){updateProjectV2View(input:$i){projectV2View{id}}}";

#[derive(Debug, Deserialize)]
struct Nodes<T> {
    nodes: Vec<T>,
}

#[derive(Debug, Deserialize)]
struct Field {
    id: String,
    name: String,
}

/// A view of a project, as GraphQL returns it.
#[derive(Debug, Deserialize)]
struct View {
    id: String,
    name: String,
    layout: String,
    filter: Option<String>,
    fields: Nodes<Field>,
}

impl View {
    fn shows(&self, column: &str) -> bool {
        self.fields.nodes.iter().any(|field| field.name == column)
    }
}

/// The fields and the views of a project.
#[derive(Debug, Deserialize)]
struct Present {
    fields: Nodes<Field>,
    views: Nodes<View>,
}

/// Rename "View 1", create the missing views and fix the others, for each
/// project. A project that is `None` does not exist yet, which happens only
/// in a dry run.
pub(super) fn views(
    run: &mut Run<'_>,
    list: &[(config::Project, Option<OrgProject>)],
) -> Result<(), Error> {
    for (wanted, project) in list {
        let specs = match wanted.kind {
            ProjectKind::Delivery => DELIVERY,
            ProjectKind::Portfolio => PORTFOLIO,
        };
        match project {
            Some(project) => project_views(run, project, specs)?,
            None => {
                for spec in specs {
                    run.say(line(Action::Create, spec.name));
                }
            }
        }
    }
    Ok(())
}

fn project_views(run: &mut Run<'_>, project: &OrgProject, specs: &[ViewSpec]) -> Result<(), Error> {
    let data = github::graphql(run.gh, READ, &json!({"p": project.id}))?;
    let node = data.get("node").cloned().unwrap_or(Value::Null);
    let present: Present =
        serde_json::from_value(node).map_err(|error| Error::Json(error.to_string()))?;
    let views = &present.views.nodes;
    let mut default = views.iter().find(|view| view.name == DEFAULT_VIEW);
    let mut renamed = None;
    for spec in specs {
        let ids = column_ids(run, &present, spec)?;
        match views.iter().find(|view| view.name == spec.name) {
            Some(view) => fix(run, view, spec, &ids)?,
            None => match default.take() {
                Some(view) => {
                    rename(run, view, spec, &ids)?;
                    renamed = Some(&view.id);
                }
                None => create(run, project, spec, &ids)?,
            },
        }
    }
    for view in views {
        let known = specs.iter().any(|spec| spec.name == view.name);
        if !known && renamed != Some(&view.id) {
            run.note(format!("extra view {}", view.name));
        }
    }
    Ok(())
}

/// The ID of each column of a view, in the order of the table. A column that
/// is not a field of the project is an error, except in a dry run, where the
/// field may be one that setup would add.
fn column_ids(
    run: &Run<'_>,
    present: &Present,
    spec: &ViewSpec,
) -> Result<Vec<(&'static str, String)>, Error> {
    let mut ids = Vec::new();
    for column in spec.columns {
        let found = present.fields.nodes.iter().find(|f| f.name == *column);
        match found {
            Some(field) => ids.push((*column, field.id.clone())),
            None if run.dry_run => {}
            None => {
                return Err(Error::Project(format!(
                    "view {}: the project has no field {column}",
                    spec.name
                )));
            }
        }
    }
    Ok(ids)
}

/// A view that exists: fix its layout, filter and columns in one update.
fn fix(
    run: &mut Run<'_>,
    view: &View,
    spec: &ViewSpec,
    ids: &[(&str, String)],
) -> Result<(), Error> {
    let mut input = json!({"viewId": view.id});
    let mut parts = Vec::new();
    if view.layout != spec.layout.graphql() {
        parts.push("layout");
        input["layout"] = spec.layout.graphql().into();
    }
    if view.filter.as_deref().unwrap_or_default() != spec.filter {
        parts.push("filter");
        input["filter"] = spec.filter.into();
    }
    if spec.columns.iter().any(|column| !view.shows(column)) {
        parts.push("columns");
        let mut visible: Vec<&str> = view.fields.nodes.iter().map(|f| f.id.as_str()).collect();
        let added = ids
            .iter()
            .filter(|(column, _)| !view.shows(column))
            .map(|(_, id)| id.as_str());
        visible.extend(added);
        input["configuration"] = json!({"visibleFieldIds": visible});
    }
    if parts.is_empty() {
        run.say(line(Action::Exists, spec.name));
        return Ok(());
    }
    run.say(line(
        Action::Update,
        &format!("{} ({})", spec.name, parts.join(", ")),
    ));
    write(run, UPDATE, &input).map(|_| ())
}

/// Turn the default view into the first missing view: name, filter and
/// columns, and the layout when it differs.
fn rename(
    run: &mut Run<'_>,
    view: &View,
    spec: &ViewSpec,
    ids: &[(&str, String)],
) -> Result<(), Error> {
    run.say(line(
        Action::Update,
        &format!("{DEFAULT_VIEW} (rename to {})", spec.name),
    ));
    let mut input = json!({
        "viewId": view.id,
        "name": spec.name,
        "filter": spec.filter,
        "configuration": {"visibleFieldIds": field_ids(ids)},
    });
    if view.layout != spec.layout.graphql() {
        input["layout"] = spec.layout.graphql().into();
    }
    write(run, UPDATE, &input).map(|_| ())
}

/// Create a view, then set its filter: `createProjectV2View` has no filter
/// (fact F1).
fn create(
    run: &mut Run<'_>,
    project: &OrgProject,
    spec: &ViewSpec,
    ids: &[(&str, String)],
) -> Result<(), Error> {
    run.say(line(Action::Create, spec.name));
    let input = json!({
        "projectId": project.id,
        "name": spec.name,
        "layout": spec.layout.graphql(),
        "configuration": {"visibleFieldIds": field_ids(ids)},
    });
    let Some(data) = write(run, CREATE, &input)? else {
        return Ok(());
    };
    if spec.filter.is_empty() {
        return Ok(());
    }
    let id = data
        .pointer("/createProjectV2View/projectV2View/id")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Json("the answer has no created view".to_owned()))?;
    write(run, UPDATE, &json!({"viewId": id, "filter": spec.filter})).map(|_| ())
}

fn field_ids<'a>(ids: &'a [(&str, String)]) -> Vec<&'a str> {
    ids.iter().map(|(_, id)| id.as_str()).collect()
}

/// Send one mutation with `input`. A dry run sends nothing and returns `None`.
fn write(run: &Run<'_>, query: &str, input: &Value) -> Result<Option<Value>, Error> {
    if run.dry_run {
        return Ok(None);
    }
    github::graphql(run.gh, query, &json!({"i": input})).map(Some)
}

fn line(action: Action, name: &str) -> Change {
    Change {
        action,
        kind: "view",
        name: name.to_owned(),
    }
}
