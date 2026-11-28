//! Parse `taskrock.toml`.

use std::path::Path;

use serde::Deserialize;

use super::{
    Ability, Config, Format, IssueType, Model, Project, ProjectKind, ReviewConfig, RoutingConfig,
    SharedMilestone, Subagent, Visibility, Workflows,
};
use crate::error::Error;

#[cfg(test)]
mod tests;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    version: u32,
    #[serde(default)]
    routing: RawRouting,
    #[serde(default)]
    review: RawReview,
    #[serde(default)]
    issue_type: Vec<RawIssueType>,
    owner_task: Option<RawFormat>,
    #[serde(default)]
    project: Vec<RawProject>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawRouting {
    #[serde(default)]
    runtimes: Vec<String>,
    #[serde(default)]
    visibility: RawVisibility,
    #[serde(default)]
    subagent: Vec<RawSubagent>,
    #[serde(default)]
    model: Vec<RawModel>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum RawVisibility {
    #[default]
    Restricted,
    All,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSubagent {
    name: String,
    #[serde(default)]
    runtimes: Vec<String>,
    #[serde(default)]
    can: Vec<RawAbility>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum RawAbility {
    Build,
    Review,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawModel {
    name: String,
    #[serde(default)]
    runtimes: Vec<String>,
    #[serde(default)]
    efforts: Vec<String>,
    #[serde(rename = "use", default)]
    usage: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, default)]
struct RawReview {
    max_rounds: u32,
    codex: Vec<String>,
}

impl Default for RawReview {
    fn default() -> Self {
        Self {
            max_rounds: ReviewConfig::default().max_rounds,
            codex: ReviewConfig::default().codex,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFormat {
    file: Vec<String>,
    build: Vec<String>,
    #[serde(default)]
    checklist: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawIssueType {
    name: String,
    file: Vec<String>,
    build: Vec<String>,
    #[serde(default)]
    checklist: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProject {
    key: String,
    title: String,
    kind: RawProjectKind,
    number: Option<u64>,
    #[serde(default)]
    repos: Vec<String>,
    #[serde(default)]
    workflows: RawWorkflows,
    #[serde(default)]
    milestone: Vec<RawMilestone>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum RawProjectKind {
    Delivery,
    Portfolio,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawWorkflows {
    auto_add_sub_issues: Option<bool>,
    item_added: Option<bool>,
    item_closed: Option<bool>,
    pr_linked: Option<bool>,
    pr_merged: Option<bool>,
    auto_close_issue: Option<bool>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMilestone {
    title: String,
    due: Option<String>,
    description: Option<String>,
}

/// Parse the text of a `taskrock.toml`.
///
/// # Errors
///
/// Returns [`Error::Config`] with the message of the TOML parser.
pub fn parse(text: &str) -> Result<Config, Error> {
    let raw: RawConfig =
        toml::from_str(text).map_err(|error| Error::Config(error.message().to_owned()))?;
    Ok(raw.into())
}

/// Read and parse a `taskrock.toml` file.
///
/// # Errors
///
/// Returns [`Error::Config`] as `<path>: <message>`.
pub fn read(path: &Path) -> Result<Config, Error> {
    let at = |message: String| Error::Config(format!("{}: {message}", path.display()));
    let text = std::fs::read_to_string(path).map_err(|error| at(error.to_string()))?;
    parse(&text).map_err(|error| match error {
        Error::Config(message) => at(message),
        other => other,
    })
}

impl From<RawConfig> for Config {
    fn from(raw: RawConfig) -> Self {
        Self {
            version: raw.version,
            routing: raw.routing.into(),
            review: ReviewConfig {
                max_rounds: raw.review.max_rounds,
                codex: raw.review.codex,
            },
            issue_types: raw.issue_type.into_iter().map(Into::into).collect(),
            owner_task: raw.owner_task.map_or_else(
                || super::default().owner_task,
                |format| Format {
                    file: format.file,
                    build: format.build,
                    checklist: format.checklist,
                },
            ),
            projects: raw.project.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<RawRouting> for RoutingConfig {
    fn from(raw: RawRouting) -> Self {
        Self {
            runtimes: raw.runtimes,
            visibility: match raw.visibility {
                RawVisibility::Restricted => Visibility::Restricted,
                RawVisibility::All => Visibility::All,
            },
            subagents: raw
                .subagent
                .into_iter()
                .map(|subagent| Subagent {
                    name: subagent.name,
                    runtimes: subagent.runtimes,
                    can: subagent
                        .can
                        .into_iter()
                        .map(|ability| match ability {
                            RawAbility::Build => Ability::Build,
                            RawAbility::Review => Ability::Review,
                        })
                        .collect(),
                })
                .collect(),
            models: raw
                .model
                .into_iter()
                .map(|model| Model {
                    name: model.name,
                    runtimes: model.runtimes,
                    efforts: model.efforts,
                    usage: model.usage,
                })
                .collect(),
        }
    }
}

impl From<RawIssueType> for IssueType {
    fn from(raw: RawIssueType) -> Self {
        Self {
            name: raw.name,
            format: Format {
                file: raw.file,
                build: raw.build,
                checklist: raw.checklist,
            },
        }
    }
}

impl From<RawProject> for Project {
    fn from(raw: RawProject) -> Self {
        Self {
            key: raw.key,
            title: raw.title,
            kind: match raw.kind {
                RawProjectKind::Delivery => ProjectKind::Delivery,
                RawProjectKind::Portfolio => ProjectKind::Portfolio,
            },
            number: raw.number,
            repos: raw.repos,
            workflows: raw.workflows.into(),
            milestones: raw
                .milestone
                .into_iter()
                .map(|milestone| SharedMilestone {
                    title: milestone.title,
                    due: milestone.due,
                    description: milestone.description,
                })
                .collect(),
        }
    }
}

impl From<RawWorkflows> for Workflows {
    fn from(raw: RawWorkflows) -> Self {
        Self {
            auto_add_sub_issues: raw.auto_add_sub_issues.unwrap_or(true),
            item_added: raw.item_added.unwrap_or(true),
            item_closed: raw.item_closed.unwrap_or(true),
            pr_linked: raw.pr_linked.unwrap_or(true),
            pr_merged: raw.pr_merged.unwrap_or(true),
            auto_close_issue: raw.auto_close_issue.unwrap_or(true),
        }
    }
}
