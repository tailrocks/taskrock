//! Step 4 of `taskrock setup`: find or create each project.

use serde_json::{Value, json};

use super::fields::organization_id;
use super::{Action, Change, Run};
use crate::config::{self, Config};
use crate::error::Error;
use crate::github;
use crate::projects::{self, Marker, OrgProject};

#[cfg(test)]
mod tests;

/// Find or create each project of the config, write its marker, set its
/// title and keep it private. `None` means the project does not exist yet,
/// which happens only in a dry run.
pub(super) fn projects(
    run: &mut Run<'_>,
    config: &Config,
) -> Result<Vec<(config::Project, Option<OrgProject>)>, Error> {
    let present = projects::list(run.gh, &run.owner)?;
    let mut owner_id = None;
    let mut found = Vec::new();
    for wanted in &config.projects {
        let project = match find(&present, wanted)? {
            Some(project) => Some(project.clone()),
            None => create(run, &mut owner_id, wanted)?,
        };
        let project = fix(run, wanted, project)?;
        found.push((wanted.clone(), project));
    }
    Ok(found)
}

/// The project with the marker key of the config, else the one with its number.
fn find<'a>(
    present: &'a [OrgProject],
    wanted: &config::Project,
) -> Result<Option<&'a OrgProject>, Error> {
    let by_marker = present
        .iter()
        .find(|project| project.marker.as_ref().is_some_and(|m| m.key == wanted.key));
    if by_marker.is_some() {
        return Ok(by_marker);
    }
    let by_number = present
        .iter()
        .find(|project| Some(project.number) == wanted.number);
    if let Some(project) = by_number
        && let Some(marker) = &project.marker
    {
        return Err(Error::Project(format!(
            "project {} has the marker of {}",
            project.number, marker.key
        )));
    }
    Ok(by_number)
}

/// Create a missing project. A dry run only says so and returns `None`.
fn create(
    run: &mut Run<'_>,
    owner_id: &mut Option<String>,
    wanted: &config::Project,
) -> Result<Option<OrgProject>, Error> {
    run.say(change(Action::Create, wanted, ""));
    if run.dry_run {
        return Ok(None);
    }
    if owner_id.is_none() {
        *owner_id = Some(organization_id(run)?);
    }
    let query = "mutation($o:ID!,$t:String!){createProjectV2(input:{ownerId:$o,title:$t}){projectV2{id number}}}";
    let data = github::graphql(run.gh, query, &json!({"o": owner_id, "t": wanted.title}))?;
    let node = data
        .pointer("/createProjectV2/projectV2")
        .ok_or_else(|| Error::Json("the answer has no created project".to_owned()))?;
    let id = node["id"]
        .as_str()
        .ok_or_else(|| Error::Json("id".to_owned()))?;
    let number = node["number"]
        .as_u64()
        .ok_or_else(|| Error::Json("number".to_owned()))?;
    Ok(Some(OrgProject {
        id: id.to_owned(),
        number,
        title: wanted.title.clone(),
        readme: String::new(),
        public: false,
        marker: None,
    }))
}

/// Write the marker, set the title and make the project private, each only
/// when needed. Returns the project as it is afterwards.
fn fix(
    run: &mut Run<'_>,
    wanted: &config::Project,
    mut project: Option<OrgProject>,
) -> Result<Option<OrgProject>, Error> {
    let marker = Marker {
        key: wanted.key.clone(),
        kind: wanted.kind,
    };
    let mut changed = false;
    if project
        .as_ref()
        .is_none_or(|p| p.marker.as_ref() != Some(&marker))
    {
        changed = true;
        run.say(change(Action::Update, wanted, " (marker)"));
        if let Some(project) = &mut project {
            project.readme = projects::with_marker(&project.readme, &marker);
            project.marker = Some(marker);
            let field = format!("readme:{}", Value::from(project.readme.as_str()));
            update(run, project, &field)?;
        }
    }
    if let Some(project) = project.as_mut().filter(|p| p.title != wanted.title) {
        changed = true;
        run.say(change(Action::Update, wanted, " (title)"));
        project.title.clone_from(&wanted.title);
        update(
            run,
            project,
            &format!("title:{}", Value::from(wanted.title.as_str())),
        )?;
    }
    if let Some(project) = project.as_mut().filter(|p| p.public) {
        changed = true;
        run.say(change(Action::Update, wanted, " (private)"));
        project.public = false;
        update(run, project, "public:false")?;
    }
    if !changed {
        run.say(change(Action::Exists, wanted, ""));
    }
    Ok(project)
}

/// `updateProjectV2` with one input field. Does nothing in a dry run.
fn update(run: &Run<'_>, project: &OrgProject, field: &str) -> Result<(), Error> {
    if run.dry_run {
        return Ok(());
    }
    let query = format!(
        "mutation($p:ID!){{updateProjectV2(input:{{projectId:$p,{field}}}){{projectV2{{id}}}}}}"
    );
    github::graphql(run.gh, &query, &json!({"p": project.id}))?;
    Ok(())
}

fn change(action: Action, wanted: &config::Project, detail: &str) -> Change {
    Change {
        action,
        kind: "project",
        name: format!("{}{detail}", wanted.title),
    }
}
