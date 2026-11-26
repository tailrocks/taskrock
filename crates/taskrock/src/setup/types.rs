//! Step 2 of `taskrock setup`: the issue types.

use super::{Action, Change, Run};
use crate::config::Config;
use crate::error::Error;
use crate::github;

/// Create each issue type of the config that the organization lacks.
pub(super) fn issue_types(run: &mut Run<'_>, config: &Config) -> Result<(), Error> {
    let path = format!("orgs/{}/issue-types", run.owner);
    let present = github::get(run.gh, &path)?;
    let names: Vec<&str> = present
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|kind| kind.get("name").and_then(serde_json::Value::as_str))
        .collect();
    for kind in &config.issue_types {
        if names.contains(&kind.name.as_str()) {
            run.say(Change {
                action: Action::Exists,
                kind: "type",
                name: kind.name.clone(),
            });
            continue;
        }
        run.say(Change {
            action: Action::Create,
            kind: "type",
            name: kind.name.clone(),
        });
        if !run.dry_run {
            let body = serde_json::json!({"name": kind.name, "is_enabled": true});
            github::send(run.gh, "POST", &path, &body)?;
        }
    }
    Ok(())
}
