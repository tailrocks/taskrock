//! A draft folder read into memory: the plan and its issues.

use crate::fields::Routing;
use crate::link::ProjectLink;

#[cfg(test)]
mod tests;

pub mod check;
pub mod coverage;
pub mod links;
pub mod parse;

/// One milestone of the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Milestone {
    pub title: String,
    pub description: Option<String>,
}

/// The plan of a draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub id: String,
    pub repo: String,
    pub project: ProjectLink,
    /// The key of the portfolio project that gets the initiatives.
    pub portfolio: Option<String>,
    pub milestones: Vec<Milestone>,
    /// The title of the milestone every created issue gets, unless it names its own.
    pub milestone: Option<String>,
}

/// One issue of a draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftIssue {
    /// The file name, for messages.
    pub file: String,
    pub key: String,
    pub title: String,
    /// The value of `type`.
    pub kind: String,
    /// The issue's own `repo`, or the plan's.
    pub repo: String,
    pub parent: Option<String>,
    pub blocked_by: Vec<String>,
    pub milestone: Option<String>,
    pub routing: Routing,
    pub body: String,
}

/// A draft: its plan and its issues.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    pub plan: Plan,
    pub issues: Vec<DraftIssue>,
}

impl Draft {
    /// The milestone title `issue` gets: its own, or the plan's.
    #[must_use]
    pub fn milestone_of<'a>(&'a self, issue: &'a DraftIssue) -> Option<&'a str> {
        issue
            .milestone
            .as_deref()
            .or(self.plan.milestone.as_deref())
    }
}

impl DraftIssue {
    /// The key of the parent when it is an issue of the draft, not a link.
    #[must_use]
    pub fn draft_parent(&self) -> Option<&str> {
        self.parent
            .as_deref()
            .filter(|parent| !links::is_link(parent))
    }
}
