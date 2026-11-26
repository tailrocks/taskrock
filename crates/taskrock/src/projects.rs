//! Delivery and portfolio projects, told apart by a marker in the README.

use serde_json::Value;

use crate::config::ProjectKind;
use crate::error::Error;
use crate::gh::Gh;
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
    let query = "query=query($o:String!){organization(login:$o){projectsV2(first:100){nodes{id number title readme public closed}}}}";
    let owner = format!("o={owner}");
    let args = ["api", "graphql", "-f", query, "-f", owner.as_str()];
    let answer = answer_of(&gh.run(&args, None)?)?;
    let nodes = answer
        .pointer("/data/organization/projectsV2/nodes")
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
    let query =
        "query=query($o:String!,$n:Int!){organization(login:$o){projectV2(number:$n){readme}}}";
    let owner = format!("o={}", project.owner);
    let number = format!("n={}", project.number);
    let args = [
        "api",
        "graphql",
        "-f",
        query,
        "-f",
        owner.as_str(),
        "-F",
        number.as_str(),
    ];
    let answer = answer_of(&gh.run(&args, None)?)?;
    let node = answer
        .pointer("/data/organization/projectV2")
        .filter(|node| !node.is_null())
        .ok_or_else(|| Error::Project(format!("no such project: {}", project.url())))?;
    Ok(parse_marker(node["readme"].as_str().unwrap_or_default())
        .map_or(ProjectKind::Delivery, |marker| marker.kind))
}

fn answer_of(stdout: &str) -> Result<Value, Error> {
    serde_json::from_str(stdout).map_err(|error| Error::Json(error.to_string()))
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
