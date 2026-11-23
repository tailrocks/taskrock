//! The issues of a project as project items.

use serde_json::Value;

use crate::error::Error;
use crate::fields::{ROUTING, Routing};
use crate::gh::Gh;
use crate::github::{get_all, send};
use crate::link::{IssueLink, ProjectLink};

#[cfg(test)]
mod tests;

/// One issue as a project shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// The ID of the project item.
    pub item_id: u64,
    pub link: IssueLink,
    /// The database ID of the issue.
    pub issue_id: u64,
    pub title: String,
    pub open: bool,
    /// The issue type name, for example `Task`.
    pub kind: Option<String>,
    /// How many sub-issues the issue has.
    pub children: u64,
    /// How many open issues block it.
    pub open_blockers: u64,
    /// How many issues it blocks.
    pub blocking: u64,
    /// The name of the Status option, for example `Todo`.
    pub status: Option<String>,
    pub parent: Option<IssueLink>,
    pub routing: Routing,
}

/// The Status field of a project and the IDs of its three options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusField {
    pub field_id: u64,
    pub todo: String,
    pub in_progress: String,
    pub done: String,
}

/// The Status field of a project with the IDs of its three options.
///
/// # Errors
///
/// Returns [`Error::Gh`] when `gh` fails, [`Error::Json`] when the answer is
/// not JSON, [`Error::Project`] when the field or an option is missing.
pub fn status_field(gh: &dyn Gh, project: &ProjectLink) -> Result<StatusField, Error> {
    parse_status_field(&field_entries(gh, project)?, project)
}

/// Every issue of a project in one listing.
///
/// # Errors
///
/// Returns [`Error::Gh`] when `gh` fails, [`Error::Json`] when the answer is
/// not JSON or a part is missing, [`Error::Project`] when a field is missing.
pub fn project_items(gh: &dyn Gh, project: &ProjectLink) -> Result<Vec<Item>, Error> {
    let entries = field_entries(gh, project)?;
    let status = field_id(&entries, project, "Status")?;
    let parent = field_id(&entries, project, "Parent issue")?;
    let path = format!(
        "{}/items?fields={status},{parent}&per_page=100",
        project.api_path()
    );
    let mut items = Vec::new();
    for value in get_all(gh, &path)? {
        if let Some(item) = parse_item(&value)? {
            items.push(item);
        }
    }
    Ok(items)
}

/// The project that holds an issue.
///
/// # Errors
///
/// Returns [`Error::Gh`] when `gh` fails, [`Error::Json`] when the answer is
/// not JSON, [`Error::Project`] when the issue is in no project or in more
/// than one project.
pub fn project_of(gh: &dyn Gh, issue: &IssueLink) -> Result<ProjectLink, Error> {
    let query = "query($o:String!,$r:String!,$n:Int!){repository(owner:$o,name:$r){issue(number:$n){projectItems(first:10){nodes{project{number owner{... on Organization{login}}}}}}}}";
    let query = format!("query={query}");
    let owner = format!("o={}", issue.owner);
    let repo = format!("r={}", issue.repo);
    let number = format!("n={}", issue.number);
    let args = [
        "api",
        "graphql",
        "-f",
        query.as_str(),
        "-f",
        owner.as_str(),
        "-f",
        repo.as_str(),
        "-F",
        number.as_str(),
    ];
    let stdout = gh.run(&args, None)?;
    let answer: Value =
        serde_json::from_str(&stdout).map_err(|error| Error::Json(error.to_string()))?;
    let nodes = answer
        .pointer("/data/repository/issue/projectItems/nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Json("nodes".to_owned()))?;
    match nodes.as_slice() {
        [node] => {
            let number = node
                .pointer("/project/number")
                .and_then(Value::as_u64)
                .ok_or_else(|| Error::Json("number".to_owned()))?;
            let owner = node
                .pointer("/project/owner/login")
                .and_then(Value::as_str)
                .ok_or_else(|| Error::Json("owner".to_owned()))?;
            Ok(ProjectLink {
                owner: owner.to_owned(),
                number,
            })
        }
        [] => Err(Error::Project(format!(
            "the issue is in no project: {}",
            issue.url()
        ))),
        _ => Err(Error::Project(format!(
            "the issue is in more than one project: {}",
            issue.url()
        ))),
    }
}

/// Set the Status option of one project item.
///
/// # Errors
///
/// Returns [`Error::Gh`] when `gh` fails, [`Error::Json`] when the answer is
/// not JSON.
pub fn set_status(
    gh: &dyn Gh,
    project: &ProjectLink,
    field: &StatusField,
    item_id: u64,
    option_id: &str,
) -> Result<(), Error> {
    let path = format!("{}/items/{item_id}", project.api_path());
    let body = serde_json::json!({"fields": [{"id": field.field_id, "value": option_id}]});
    send(gh, "PATCH", &path, &body)?;
    Ok(())
}

fn field_entries(gh: &dyn Gh, project: &ProjectLink) -> Result<Vec<Value>, Error> {
    get_all(gh, &format!("{}/fields?per_page=100", project.api_path()))
}

fn field_id(entries: &[Value], project: &ProjectLink, name: &str) -> Result<u64, Error> {
    entries
        .iter()
        .find(|entry| entry["name"] == name)
        .and_then(|entry| entry["id"].as_u64())
        .ok_or_else(|| Error::Project(format!("project {} has no {name} field", project.url())))
}

fn parse_status_field(entries: &[Value], project: &ProjectLink) -> Result<StatusField, Error> {
    let status = entries
        .iter()
        .find(|entry| entry["name"] == "Status")
        .ok_or_else(|| Error::Project(format!("project {} has no Status field", project.url())))?;
    let options = status["options"].as_array().ok_or_else(|| {
        Error::Project(format!(
            "Status field of project {} has no options",
            project.url()
        ))
    })?;
    let option = |want: &str| {
        options
            .iter()
            .find(|item| {
                item["name"]["raw"]
                    .as_str()
                    .is_some_and(|name| name.eq_ignore_ascii_case(want))
            })
            .and_then(|item| item["id"].as_str())
            .map(str::to_owned)
            .ok_or_else(|| {
                Error::Project(format!(
                    "Status field of project {} has no {want} option",
                    project.url()
                ))
            })
    };
    Ok(StatusField {
        field_id: status["id"]
            .as_u64()
            .ok_or_else(|| Error::Json("id".to_owned()))?,
        todo: option("Todo")?,
        in_progress: option("In Progress")?,
        done: option("Done")?,
    })
}

fn parse_item(value: &Value) -> Result<Option<Item>, Error> {
    if required_text(value, "content_type", "content_type")? != "Issue" {
        return Ok(None);
    }
    let content = value
        .get("content")
        .ok_or_else(|| Error::Json("content".to_owned()))?;
    let link = required_text(content, "html_url", "link")?.parse::<IssueLink>()?;
    let summary = content
        .get("sub_issues_summary")
        .ok_or_else(|| Error::Json("children".to_owned()))?;
    let dependencies = content
        .get("issue_dependencies_summary")
        .ok_or_else(|| Error::Json("open_blockers".to_owned()))?;
    Ok(Some(Item {
        item_id: required_id(value, "id", "item_id")?,
        link,
        issue_id: required_id(content, "id", "issue_id")?,
        title: required_text(content, "title", "title")?.to_owned(),
        open: required_text(content, "state", "open")? == "open",
        kind: kind_of(content)?,
        children: required_id(summary, "total", "children")?,
        open_blockers: required_id(dependencies, "blocked_by", "open_blockers")?,
        blocking: required_id(dependencies, "blocking", "blocking")?,
        status: status_of(value)?,
        parent: parent_of(value)?,
        routing: routing_of(content),
    }))
}

fn kind_of(content: &Value) -> Result<Option<String>, Error> {
    match content.get("type") {
        None | Some(Value::Null) => Ok(None),
        Some(kind) => kind
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| Error::Json("kind".to_owned()))
            .map(Some),
    }
}

fn status_of(value: &Value) -> Result<Option<String>, Error> {
    let Some(entry) = named_field(value, "Status") else {
        return Ok(None);
    };
    match entry.get("value") {
        None | Some(Value::Null) => Ok(None),
        Some(field) => field
            .get("name")
            .and_then(|name| name.get("raw"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| Error::Json("status".to_owned()))
            .map(Some),
    }
}

fn parent_of(value: &Value) -> Result<Option<IssueLink>, Error> {
    let Some(entry) = named_field(value, "Parent issue") else {
        return Ok(None);
    };
    match entry.get("value") {
        None | Some(Value::Null) => Ok(None),
        Some(field) => field
            .get("url")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::Json("parent".to_owned()))?
            .parse::<IssueLink>()
            .map(Some),
    }
}

fn routing_of(content: &Value) -> Routing {
    let mut routing = Routing::default();
    let values = content.get("issue_field_values").and_then(Value::as_array);
    for entry in values.into_iter().flatten() {
        let Some(name) = entry.get("issue_field_name").and_then(Value::as_str) else {
            continue;
        };
        if !ROUTING.contains(&name) {
            continue;
        }
        if let Some(value) = entry
            .get("single_select_option")
            .and_then(|option| option.get("name"))
            .and_then(Value::as_str)
        {
            routing.set(name, value);
        }
    }
    routing
}

fn named_field<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    value
        .get("fields")?
        .as_array()?
        .iter()
        .find(|entry| entry["name"] == name)
}

fn required_id(value: &Value, key: &str, part: &str) -> Result<u64, Error> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| Error::Json(part.to_owned()))
}

fn required_text<'a>(value: &'a Value, key: &str, part: &str) -> Result<&'a str, Error> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Json(part.to_owned()))
}
