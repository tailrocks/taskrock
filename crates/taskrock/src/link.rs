//! The links a person pastes: GitHub issues and projects.

use std::str::FromStr;

use crate::error::Error;

#[cfg(test)]
mod pull_tests;
#[cfg(test)]
mod tests;

/// One issue on GitHub.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IssueLink {
    pub owner: String,
    pub repo: String,
    pub number: u64,
}

/// One pull request on GitHub.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PullLink {
    pub owner: String,
    pub repo: String,
    pub number: u64,
}

/// One project of an organization.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProjectLink {
    pub owner: String,
    pub number: u64,
}

/// What a pasted link points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    Issue(IssueLink),
    Project(ProjectLink),
}

impl FromStr for Link {
    type Err = Error;

    /// Parse one of the links taskrock understands.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Link`] when the text is not a link taskrock understands.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let text = s.trim();
        if let Ok(issue) = text.parse::<IssueLink>() {
            return Ok(Self::Issue(issue));
        }
        if let Some(project) = parse_project(text) {
            return Ok(Self::Project(project));
        }
        if text.starts_with("https://github.com/users/") {
            return Err(Error::Link(
                "projects of a personal account are not supported".to_owned(),
            ));
        }
        if text.contains("/pull/") || text.contains("/pulls/") {
            return Err(Error::Link(
                "this is a pull request, not an issue".to_owned(),
            ));
        }
        Err(Error::Link(text.to_owned()))
    }
}

impl FromStr for IssueLink {
    type Err = Error;

    /// Parse one of the three issue forms.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Link`] when the text is not an issue link.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let text = s.trim();
        parse_issue(text).ok_or_else(|| Error::Link(text.to_owned()))
    }
}

impl FromStr for PullLink {
    type Err = Error;

    /// Parse one of the two pull request forms.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Link`] when the text is not a pull request link.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let text = s.trim();
        parse_pull(text).ok_or_else(|| Error::Link(format!("not a pull request link: {text}")))
    }
}

impl IssueLink {
    /// `https://github.com/{owner}/{repo}/issues/{number}`
    #[must_use]
    pub fn url(&self) -> String {
        format!(
            "https://github.com/{}/{}/issues/{}",
            self.owner, self.repo, self.number
        )
    }

    /// `repos/{owner}/{repo}/issues/{number}`
    #[must_use]
    pub fn api_path(&self) -> String {
        format!("repos/{}/{}/issues/{}", self.owner, self.repo, self.number)
    }

    /// `repos/{owner}/{repo}`
    #[must_use]
    pub fn repo_path(&self) -> String {
        format!("repos/{}/{}", self.owner, self.repo)
    }

    /// `feat/issue-{number}`
    #[must_use]
    pub fn branch(&self) -> String {
        format!("feat/issue-{}", self.number)
    }
}

impl PullLink {
    /// `https://github.com/{owner}/{repo}/pull/{number}`
    #[must_use]
    pub fn url(&self) -> String {
        format!(
            "https://github.com/{}/{}/pull/{}",
            self.owner, self.repo, self.number
        )
    }

    /// `repos/{owner}/{repo}/pulls/{number}`
    #[must_use]
    pub fn api_path(&self) -> String {
        format!("repos/{}/{}/pulls/{}", self.owner, self.repo, self.number)
    }

    /// `repos/{owner}/{repo}`
    #[must_use]
    pub fn repo_path(&self) -> String {
        format!("repos/{}/{}", self.owner, self.repo)
    }
}

impl ProjectLink {
    /// `https://github.com/orgs/{owner}/projects/{number}`
    #[must_use]
    pub fn url(&self) -> String {
        format!(
            "https://github.com/orgs/{}/projects/{}",
            self.owner, self.number
        )
    }

    /// `orgs/{owner}/projectsV2/{number}`
    #[must_use]
    pub fn api_path(&self) -> String {
        format!("orgs/{}/projectsV2/{}", self.owner, self.number)
    }
}

fn parse_issue(text: &str) -> Option<IssueLink> {
    if let Some(rest) = text.strip_prefix("https://github.com/") {
        parse_slashed_issue(rest)
    } else if let Some(rest) = text.strip_prefix("https://api.github.com/repos/") {
        parse_slashed_issue(rest)
    } else {
        parse_short_issue(text)
    }
}

fn parse_slashed_issue(rest: &str) -> Option<IssueLink> {
    let mut parts = cut_tail(rest).split('/');
    let owner = parse_name(parts.next()?)?.to_owned();
    let repo = parse_name(parts.next()?)?.to_owned();
    if parts.next()? != "issues" {
        return None;
    }
    let number = parse_number(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    Some(IssueLink {
        owner,
        repo,
        number,
    })
}

fn parse_short_issue(text: &str) -> Option<IssueLink> {
    let (owner, repo, number) = parse_short(text)?;
    Some(IssueLink {
        owner,
        repo,
        number,
    })
}

fn parse_pull(text: &str) -> Option<PullLink> {
    let (owner, repo, number) = match text.strip_prefix("https://github.com/") {
        Some(rest) => parse_slashed_pull(rest)?,
        None => parse_short(text)?,
    };
    Some(PullLink {
        owner,
        repo,
        number,
    })
}

fn parse_slashed_pull(rest: &str) -> Option<(String, String, u64)> {
    let mut parts = cut_tail(rest).split('/');
    let owner = parse_name(parts.next()?)?.to_owned();
    let repo = parse_name(parts.next()?)?.to_owned();
    if parts.next()? != "pull" {
        return None;
    }
    let number = parse_number(parts.next()?)?;
    match parts.next() {
        None | Some("files" | "commits" | "checks") if parts.next().is_none() => {
            Some((owner, repo, number))
        }
        _ => None,
    }
}

fn parse_short(text: &str) -> Option<(String, String, u64)> {
    let (slug, number) = text.split_once('#')?;
    let (owner, repo) = slug.split_once('/')?;
    let owner = parse_name(owner)?.to_owned();
    let repo = parse_name(repo)?.to_owned();
    let number = parse_number(cut_tail(number))?;
    Some((owner, repo, number))
}

fn parse_project(text: &str) -> Option<ProjectLink> {
    let rest = text.strip_prefix("https://github.com/orgs/")?;
    let mut parts = cut_tail(rest).split('/');
    let owner = parse_name(parts.next()?)?.to_owned();
    if parts.next()? != "projects" {
        return None;
    }
    let number = parse_number(parts.next()?)?;
    if let Some(views) = parts.next() {
        if views != "views" || parse_number(parts.next()?).is_none() {
            return None;
        }
        if parts.next().is_some() {
            return None;
        }
    }
    Some(ProjectLink { owner, number })
}

fn parse_name(text: &str) -> Option<&str> {
    let bad = |byte: u8| byte.is_ascii_whitespace() || matches!(byte, b'/' | b'?' | b'#');
    if text.is_empty() || text.bytes().any(bad) {
        return None;
    }
    Some(text)
}

fn parse_number(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse::<u64>().ok()
}

fn cut_tail(text: &str) -> &str {
    let mut end = text.len();
    if let Some(index) = text.find('?') {
        end = end.min(index);
    }
    if let Some(index) = text.find('#') {
        end = end.min(index);
    }
    text[..end].trim_end_matches('/')
}
