//! The config of an organization: `taskrock.toml`.

pub mod check;
pub mod load;
pub mod parse;

#[cfg(test)]
mod tests;

/// The config of an organization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub version: u32,
    pub routing: RoutingConfig,
    pub review: ReviewConfig,
    /// TOML `[[issue_type]]`.
    pub issue_types: Vec<IssueType>,
    /// TOML `[owner_task]`.
    pub owner_task: Format,
    /// TOML `[[project]]`.
    pub projects: Vec<Project>,
}

/// Rules for the planner: who builds, where, and with which model.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RoutingConfig {
    pub runtimes: Vec<String>,
    pub visibility: Visibility,
    /// TOML `[[routing.subagent]]`.
    pub subagents: Vec<Subagent>,
    /// TOML `[[routing.model]]`.
    pub models: Vec<Model>,
}

/// Who sees the routing values.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Visibility {
    #[default]
    Restricted,
    All,
}

/// A subagent: where it is installed and what it can do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subagent {
    pub name: String,
    pub runtimes: Vec<String>,
    pub can: Vec<Ability>,
}

/// What a subagent can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ability {
    Build,
    Review,
}

/// A model: where it runs, its efforts, and when the planner uses it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub name: String,
    pub runtimes: Vec<String>,
    pub efforts: Vec<String>,
    /// TOML key `use`.
    pub usage: String,
}

/// The review rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewConfig {
    /// reviewed heads before the task goes to the owner.
    pub max_rounds: u32,
}

/// An issue type and its format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueType {
    pub name: String,
    pub format: Format,
}

/// The sections of an issue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Format {
    /// Needed to create the issue.
    pub file: Vec<String>,
    /// Needed to take the issue.
    pub build: Vec<String>,
    /// Sections that are checklists.
    pub checklist: Vec<String>,
}

/// A project of the organization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub key: String,
    pub title: String,
    pub kind: ProjectKind,
    pub number: Option<u64>,
    /// Repository names, without the owner.
    pub repos: Vec<String>,
    /// TOML `[project.workflows]`.
    pub workflows: Workflows,
    /// TOML `[[project.milestone]]`.
    pub milestones: Vec<SharedMilestone>,
}

/// The kind of a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectKind {
    Delivery,
    Portfolio,
}

/// The built-in workflows of a project. Every one is on by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one flag per GitHub workflow, as the config file has them"
)]
pub struct Workflows {
    pub auto_add_sub_issues: bool,
    pub item_added: bool,
    pub item_closed: bool,
    pub pr_linked: bool,
    pub pr_merged: bool,
    pub auto_close_issue: bool,
}

impl Default for Workflows {
    fn default() -> Self {
        Self {
            auto_add_sub_issues: true,
            item_added: true,
            item_closed: true,
            pr_linked: true,
            pr_merged: true,
            auto_close_issue: true,
        }
    }
}

/// A milestone made in every repository of a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedMilestone {
    pub title: String,
    pub due: Option<String>,
    pub description: Option<String>,
}

impl Config {
    /// The format of an issue: the owner-task format when `owner` is true,
    /// else the format of the type named `kind`.
    #[must_use]
    pub fn format(&self, kind: &str, owner: bool) -> Option<&Format> {
        if owner {
            return Some(&self.owner_task);
        }
        self.issue_types
            .iter()
            .find(|issue_type| issue_type.name == kind)
            .map(|issue_type| &issue_type.format)
    }

    /// The names of the issue types, in file order.
    #[must_use]
    pub fn type_names(&self) -> Vec<&str> {
        self.issue_types
            .iter()
            .map(|issue_type| issue_type.name.as_str())
            .collect()
    }
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self { max_rounds: 3 }
    }
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

fn format(file: &[&str], build: &[&str]) -> Format {
    Format {
        file: strings(file),
        build: strings(build),
        checklist: strings(&["Done when"]),
    }
}

fn issue_type(name: &str, format: Format) -> IssueType {
    IssueType {
        name: name.to_owned(),
        format,
    }
}

/// The built-in config: version 1, empty routing, `max_rounds` 3, no
/// projects, and the default formats for Initiative, Epic, Task, Bug,
/// Feature and the owner task.
#[must_use]
pub fn default() -> Config {
    let initiative = [
        "Outcome",
        "Why",
        "Not yet specified",
        "Out of scope",
        "Done when",
    ];
    let epic = [
        "Outcome",
        "Decisions already made",
        "Not yet specified",
        "Out of scope",
        "Done when",
    ];
    let task = [
        "Outcome",
        "Decisions already made",
        "Steps",
        "Done when",
        "Out of scope",
    ];
    let bug = [
        "What happens",
        "Expected",
        "How to reproduce",
        "Environment",
    ];
    let bug_build = [
        "What happens",
        "Expected",
        "How to reproduce",
        "Environment",
        "Cause",
        "Decisions already made",
        "Steps",
        "Done when",
        "Out of scope",
    ];
    let feature = ["Problem", "Proposed behavior", "Workaround today"];
    let feature_build = [
        "Problem",
        "Proposed behavior",
        "Workaround today",
        "Decisions already made",
        "Steps",
        "Done when",
        "Out of scope",
    ];
    let owner = [
        "Outcome",
        "Question",
        "Why it matters",
        "Recommended answer",
        "Done when",
    ];
    Config {
        version: 1,
        routing: RoutingConfig::default(),
        review: ReviewConfig::default(),
        issue_types: vec![
            issue_type("Initiative", format(&initiative, &initiative)),
            issue_type("Epic", format(&epic, &epic)),
            issue_type("Task", format(&task, &task)),
            issue_type("Bug", format(&bug, &bug_build)),
            issue_type("Feature", format(&feature, &feature_build)),
        ],
        owner_task: format(&owner, &owner),
        projects: Vec::new(),
    }
}
