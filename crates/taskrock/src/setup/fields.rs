//! Step 3 of `taskrock setup`: the eight routing fields.

use serde::Deserialize;
use serde_json::{Value, json};

use super::{Action, Change, Run};
use crate::config::{Ability, Config, Visibility};
use crate::error::Error;
use crate::github;

#[cfg(test)]
mod tests;

/// The most issue fields an organization can have.
const LIMIT: usize = 25;
/// The option names of GitHub's default `Effort` field.
const SIZES: [&str; 5] = ["XS", "S", "M", "L", "XL"];

/// An issue field as `GET orgs/<owner>/issue-fields` returns it.
#[derive(Debug, Deserialize)]
struct Field {
    id: u64,
    name: String,
    #[serde(default)]
    visibility: String,
    #[serde(default)]
    options: Vec<FieldOption>,
}

/// An option of an issue field.
#[derive(Debug, Deserialize)]
struct FieldOption {
    id: u64,
    name: String,
    color: Option<String>,
    priority: u64,
}

/// The eight routing fields with their wanted options, in this order.
pub(super) fn wanted(config: &Config) -> Vec<(&'static str, Vec<String>)> {
    let routing = &config.routing;
    let can = |ability: Ability| -> Vec<String> {
        routing
            .subagents
            .iter()
            .filter(|subagent| subagent.can.contains(&ability))
            .map(|subagent| subagent.name.clone())
            .collect()
    };
    let mut agents = vec!["Owner".to_owned()];
    agents.extend(can(Ability::Build));
    let reviewers = can(Ability::Review);
    let runtimes = routing.runtimes.clone();
    let models: Vec<String> = routing.models.iter().map(|m| m.name.clone()).collect();
    let mut efforts: Vec<String> = Vec::new();
    for effort in routing.models.iter().flat_map(|model| &model.efforts) {
        if !efforts.contains(effort) {
            efforts.push(effort.clone());
        }
    }
    vec![
        ("Agent", agents),
        ("Review agent", reviewers),
        ("Runtime", runtimes.clone()),
        ("Review runtime", runtimes),
        ("Model", models.clone()),
        ("Review model", models),
        ("Effort", efforts.clone()),
        ("Review effort", efforts),
    ]
}

/// The description of a new routing field.
fn description(name: &str) -> &'static str {
    match name {
        "Agent" => "Who builds the task: a role, or Owner for work only the owner can do.",
        "Runtime" => "The agent CLI that builds the task.",
        "Model" => "The model that builds the task.",
        "Effort" => "The reasoning effort of the builder.",
        "Review agent" => "Who reviews the pull request before it merges.",
        "Review runtime" => "The agent CLI that reviews the pull request.",
        "Review model" => "The model that reviews the pull request.",
        "Review effort" => "The reasoning effort of the reviewer.",
        _ => "",
    }
}

/// Create the missing routing fields and add the missing options.
pub(super) fn routing_fields(run: &mut Run<'_>, config: &Config) -> Result<(), Error> {
    let path = format!("orgs/{}/issue-fields", run.owner);
    let mut present = read_fields(run, &path)?;
    let rename = rename_target(&present)?;
    let wanted = wanted(config);
    let missing = wanted
        .iter()
        .filter(|(name, _)| {
            let found = present.iter().position(|field| field.name == *name);
            found.is_none() || found == rename
        })
        .count();
    let total = present.len() + missing;
    if total > LIMIT {
        return Err(Error::Project(format!(
            "the organization would have {total} issue fields. The limit is {LIMIT}"
        )));
    }
    if let Some(index) = rename {
        run.say(Change {
            action: Action::Update,
            kind: "field",
            name: "Effort (rename to Size)".to_owned(),
        });
        if !run.dry_run {
            let target = format!("{path}/{}", present[index].id);
            github::send(run.gh, "PATCH", &target, &json!({"name": "Size"}))?;
        }
        "Size".clone_into(&mut present[index].name);
    }
    let mut owner_id = None;
    for (name, options) in &wanted {
        match present.iter().find(|field| field.name == *name) {
            None => create(run, &mut owner_id, name, options)?,
            Some(field) => update(run, &path, config, field, options)?,
        }
    }
    Ok(())
}

fn read_fields(run: &Run<'_>, path: &str) -> Result<Vec<Field>, Error> {
    let answer = github::get(run.gh, path)?;
    serde_json::from_value(answer).map_err(|error| Error::Json(error.to_string()))
}

/// The index of the `Effort` field that holds sizes, when it must be renamed.
fn rename_target(present: &[Field]) -> Result<Option<usize>, Error> {
    let Some(index) = present.iter().position(|field| field.name == "Effort") else {
        return Ok(None);
    };
    let sizes = present[index]
        .options
        .iter()
        .all(|option| SIZES.contains(&option.name.as_str()));
    if !sizes {
        return Ok(None);
    }
    if present.iter().any(|field| field.name == "Size") {
        return Err(Error::Project(
            "the field Effort holds sizes and a field Size exists. Rename one of them".to_owned(),
        ));
    }
    Ok(Some(index))
}

/// Create a missing field with GraphQL, as the old script did.
fn create(
    run: &mut Run<'_>,
    owner_id: &mut Option<String>,
    name: &str,
    options: &[String],
) -> Result<(), Error> {
    run.say(Change {
        action: Action::Create,
        kind: "field",
        name: name.to_owned(),
    });
    if run.dry_run {
        return Ok(());
    }
    if owner_id.is_none() {
        *owner_id = Some(organization_id(run)?);
    }
    let listed: Vec<String> = options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            format!(
                "{{name:{},color:GRAY,priority:{}}}",
                Value::from(option.as_str()),
                index + 1
            )
        })
        .collect();
    let query = format!(
        "mutation($o:ID!,$n:String!,$d:String!){{createIssueField(input:{{ownerId:$o,name:$n,description:$d,dataType:SINGLE_SELECT,options:[{}]}}){{issueField{{... on IssueFieldSingleSelect{{name}}}}}}}}",
        listed.join(",")
    );
    let variables = json!({"o": owner_id, "n": name, "d": description(name)});
    github::graphql(run.gh, &query, &variables)?;
    Ok(())
}

fn organization_id(run: &Run<'_>) -> Result<String, Error> {
    let query = "query($o:String!){organization(login:$o){id}}";
    let data = github::graphql(run.gh, query, &json!({"o": run.owner}))?;
    data.pointer("/organization/id")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| Error::Json("the answer has no organization id".to_owned()))
}

/// Add the missing options and fix the visibility of an existing field.
fn update(
    run: &mut Run<'_>,
    path: &str,
    config: &Config,
    field: &Field,
    options: &[String],
) -> Result<(), Error> {
    let target = format!("{path}/{}", field.id);
    let new: Vec<&String> = options
        .iter()
        .filter(|option| !field.options.iter().any(|have| have.name == **option))
        .collect();
    let mut changed = false;
    if !new.is_empty() {
        changed = true;
        let names: Vec<&str> = new.iter().map(|option| option.as_str()).collect();
        run.say(Change {
            action: Action::Update,
            kind: "field",
            name: format!("{} (add {})", field.name, names.join(", ")),
        });
        if !run.dry_run {
            github::send(run.gh, "PATCH", &target, &add_options(field, &new))?;
        }
    }
    let visibility = match config.routing.visibility {
        Visibility::Restricted => "organization_members_only",
        Visibility::All => "all",
    };
    if field.visibility != visibility {
        changed = true;
        run.say(Change {
            action: Action::Update,
            kind: "field",
            name: format!("{} (visibility)", field.name),
        });
        if !run.dry_run {
            github::send(run.gh, "PATCH", &target, &json!({"visibility": visibility}))?;
        }
    }
    if !changed {
        run.say(Change {
            action: Action::Exists,
            kind: "field",
            name: field.name.clone(),
        });
    }
    Ok(())
}

/// The PATCH body: every existing option with its id, then the new ones.
fn add_options(field: &Field, new: &[&String]) -> Value {
    let mut options: Vec<Value> = field
        .options
        .iter()
        .map(|option| {
            json!({
                "id": option.id,
                "name": option.name,
                "color": option.color,
                "priority": option.priority,
            })
        })
        .collect();
    let top = field.options.iter().map(|o| o.priority).max().unwrap_or(0);
    for (offset, name) in (1..).zip(new) {
        options.push(json!({"name": name, "color": "gray", "priority": top + offset}));
    }
    json!({"options": options})
}
