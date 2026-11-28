//! Delivery and portfolio projects, told apart by a marker in the README.

use serde_json::Value;

use crate::config::ProjectKind;
use crate::error::Error;
use crate::gh::Gh;
use crate::github;
use crate::link::ProjectLink;

#[cfg(test)]
mod tests;

const PREFIX: &str = "<!-- taskrock:project ";
const SUFFIX: &str = " -->";

/// The marker at the end of a project README.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marker {
    pub key: String,
    pub kind: ProjectKind,
}

/// One open project of an organization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgProject {
    pub id: String,
    pub number: u64,
    pub title: String,
    pub readme: String,
    pub public: bool,
    pub marker: Option<Marker>,
}

/// `<!-- taskrock:project key=<key> kind=<delivery|portfolio> -->`
#[must_use]
pub fn marker_line(marker: &Marker) -> String {
    let kind = match marker.kind {
        ProjectKind::Delivery => "delivery",
        ProjectKind::Portfolio => "portfolio",
    };
    format!("{PREFIX}key={} kind={kind}{SUFFIX}", marker.key)
}

/// The last marker line of the README, if any.
#[must_use]
pub fn parse_marker(readme: &str) -> Option<Marker> {
    readme.lines().rev().find_map(parse_line)
}

/// The README with every old marker line removed and `marker_line` appended
/// after one empty line, trimmed.
#[must_use]
pub fn with_marker(readme: &str, marker: &Marker) -> String {
    let kept: Vec<&str> = readme
        .lines()
        .filter(|line| parse_line(line).is_none())
        .collect();
    let body = kept.join("\n");
    format!("{}\n\n{}", body.trim(), marker_line(marker))
        .trim()
        .to_owned()
}

/// The open projects of an organization, from one GraphQL call.
///
/// # Errors
///
/// Returns [`Error::Gh`] when `gh` fails, [`Error::Json`] when the answer is
/// not JSON or a part is missing.
pub fn list(gh: &dyn Gh, owner: &str) -> Result<Vec<OrgProject>, Error> {
    let query = "query($o:String!){repositoryOwner(login:$o){... on Organization{projectsV2(first:100){nodes{id number title readme public closed}}}}}";
    let data = github::graphql(gh, query, &serde_json::json!({"o": owner}))?;
    let nodes = data
        .pointer("/repositoryOwner/projectsV2/nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Json("nodes".to_owned()))?;
    let mut projects = Vec::new();
    for node in nodes {
        if node["closed"].as_bool() == Some(true) {
            continue;
        }
        projects.push(parse_project(node)?);
    }
    Ok(projects)
}

/// The kind of one project. No marker means Delivery.
///
/// # Errors
///
/// Returns [`Error::Gh`] when `gh` fails, [`Error::Json`] when the answer is
/// not JSON, [`Error::Project`] when the project does not exist.
pub fn kind_of(gh: &dyn Gh, project: &ProjectLink) -> Result<ProjectKind, Error> {
    let query = "query($o:String!,$n:Int!){repositoryOwner(login:$o){... on Organization{projectV2(number:$n){readme}}}}";
    let variables = serde_json::json!({"o": project.owner, "n": project.number});
    let data = github::graphql(gh, query, &variables)?;
    let node = data
        .pointer("/repositoryOwner/projectV2")
        .filter(|node| !node.is_null())
        .ok_or_else(|| Error::Project(format!("no such project: {}", project.url())))?;
    Ok(parse_marker(node["readme"].as_str().unwrap_or_default())
        .map_or(ProjectKind::Delivery, |marker| marker.kind))
}

fn parse_project(node: &Value) -> Result<OrgProject, Error> {
    let text = |key: &str| {
        node[key]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| Error::Json(key.to_owned()))
    };
    let readme = node["readme"].as_str().unwrap_or_default().to_owned();
    Ok(OrgProject {
        id: text("id")?,
        number: node["number"]
            .as_u64()
            .ok_or_else(|| Error::Json("number".to_owned()))?,
        title: text("title")?,
        marker: parse_marker(&readme),
        readme,
        public: node["public"].as_bool().unwrap_or(false),
    })
}

fn parse_line(line: &str) -> Option<Marker> {
    let inner = line.trim().strip_prefix(PREFIX)?.strip_suffix(SUFFIX)?;
    let mut words = inner.split_whitespace();
    let key = words.next()?.strip_prefix("key=")?;
    let kind = match words.next()?.strip_prefix("kind=")? {
        "delivery" => ProjectKind::Delivery,
        "portfolio" => ProjectKind::Portfolio,
        _ => return None,
    };
    if key.is_empty() || words.next().is_some() {
        return None;
    }
    Some(Marker {
        key: key.to_owned(),
        kind,
    })
}
