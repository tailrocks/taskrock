//! Step 7 of `taskrock setup`: link each repository of the config to its
//! project.

use serde_json::{Value, json};

use super::{Action, Change, Run};
use crate::config;
use crate::error::Error;
use crate::github;
use crate::projects::OrgProject;

#[cfg(test)]
mod tests;

const LINKED: &str =
    "query($p:ID!){node(id:$p){... on ProjectV2{repositories(first:100){nodes{name}}}}}";
const LINK: &str = "mutation($p:ID!,$r:ID!){linkProjectV2ToRepository(input:{projectId:$p,repositoryId:$r}){clientMutationId}}";

/// Link each repository of the config to its project. A repository that is
/// linked and not in the config is noted and kept. A project that is `None`
/// does not exist yet, which happens only in a dry run: nothing is read.
pub(super) fn repos(
    run: &mut Run<'_>,
    list: &[(config::Project, Option<OrgProject>)],
) -> Result<(), Error> {
    for (wanted, project) in list {
        let linked = match project {
            Some(project) => linked(run, project)?,
            None => Vec::new(),
        };
        for repo in &wanted.repos {
            let action = if linked.contains(repo) {
                Action::Exists
            } else {
                Action::Create
            };
            run.say(Change {
                action,
                kind: "link",
                name: format!("{} {}/{repo}", wanted.title, run.owner),
            });
            if let (Action::Create, false, Some(project)) = (action, run.dry_run, project) {
                link(run, project, repo)?;
            }
        }
        for repo in linked.iter().filter(|name| !wanted.repos.contains(name)) {
            run.note(format!("extra link {} {}/{repo}", wanted.title, run.owner));
        }
    }
    Ok(())
}

/// The names of the repositories linked to the project.
fn linked(run: &Run<'_>, project: &OrgProject) -> Result<Vec<String>, Error> {
    let data = github::graphql(run.gh, LINKED, &json!({"p": project.id}))?;
    let nodes = data
        .pointer("/node/repositories/nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Json("the answer has no linked repositories".to_owned()))?;
    Ok(nodes
        .iter()
        .filter_map(|node| node.get("name").and_then(Value::as_str).map(str::to_owned))
        .collect())
}

/// `linkProjectV2ToRepository`, with the `node_id` of the repository.
fn link(run: &Run<'_>, project: &OrgProject, repo: &str) -> Result<(), Error> {
    let repository = github::get(run.gh, &format!("repos/{}/{repo}", run.owner))?;
    let node_id = repository
        .get("node_id")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Json(format!("the repository {repo} has no node_id")))?;
    github::graphql(run.gh, LINK, &json!({"p": project.id, "r": node_id}))?;
    Ok(())
}
