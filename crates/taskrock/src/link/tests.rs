//! Tests for links.

use std::str::FromStr as _;

use super::{IssueLink, Link, ProjectLink};
use crate::error::Error;

fn issue() -> IssueLink {
    IssueLink {
        owner: "tailrocks".into(),
        repo: "holla".into(),
        number: 912,
    }
}

fn project() -> ProjectLink {
    ProjectLink {
        owner: "tailrocks".into(),
        number: 3,
    }
}

fn refuse(text: &str) -> String {
    let Err(Error::Link(message)) = Link::from_str(text) else {
        panic!("{text} is refused");
    };
    message
}

#[test]
fn github_issue_url() {
    let link = Link::from_str("https://github.com/tailrocks/holla/issues/912");
    assert_eq!(link.expect("the issue URL parses"), Link::Issue(issue()));
}

#[test]
fn api_issue_url() {
    let link = Link::from_str("https://api.github.com/repos/tailrocks/holla/issues/912");
    let parsed = link.expect("the API issue URL parses");
    assert_eq!(parsed, Link::Issue(issue()));
}

#[test]
fn short_issue_text() {
    let link = Link::from_str("tailrocks/holla#912");
    assert_eq!(link.expect("the short form parses"), Link::Issue(issue()));
}

#[test]
fn project_url() {
    let link = Link::from_str("https://github.com/orgs/tailrocks/projects/3");
    let parsed = link.expect("the project URL parses");
    assert_eq!(parsed, Link::Project(project()));
}

#[test]
fn project_view_url() {
    let link = Link::from_str("https://github.com/orgs/tailrocks/projects/3/views/2");
    assert_eq!(link.expect("the view URL parses"), Link::Project(project()));
}

#[test]
fn personal_projects_are_refused() {
    let message = refuse("https://github.com/users/donbeave/projects/1");
    let expected = "projects of a personal account are not supported";
    assert_eq!(message, expected);
}

#[test]
fn pull_requests_are_refused() {
    let message = refuse("https://github.com/tailrocks/holla/pull/4");
    let expected = "this is a pull request, not an issue";
    assert_eq!(message, expected);
}

#[test]
fn anything_else_is_refused() {
    for text in [
        "",
        "holla#912",
        "tailrocks/holla#abc",
        "https://github.com/tailrocks/holla",
        "https://gitlab.com/a/b/issues/1",
        "https://github.com/tailrocks/holla/issues/0x1",
    ] {
        let message = refuse(text);
        let expected = format!("not a link taskrock understands: {text}");
        assert_eq!(message, expected);
    }
}

#[test]
fn spaces_and_suffixes_are_ignored() {
    for text in [
        "  https://github.com/tailrocks/holla/issues/912  ",
        "https://github.com/tailrocks/holla/issues/912/",
        "https://github.com/tailrocks/holla/issues/912?foo=bar",
        "https://github.com/tailrocks/holla/issues/912#readme",
        "  tailrocks/holla#912 ",
    ] {
        let parsed = Link::from_str(text).expect("affixes are ignored");
        assert_eq!(parsed, Link::Issue(issue()));
    }
}

#[test]
fn issue_from_str_handles_every_form() {
    for text in [
        "https://github.com/tailrocks/holla/issues/912",
        "https://api.github.com/repos/tailrocks/holla/issues/912",
        "tailrocks/holla#912",
    ] {
        assert_eq!(IssueLink::from_str(text).expect("the form parses"), issue());
    }
    let project = "https://github.com/orgs/tailrocks/projects/3";
    assert!(IssueLink::from_str(project).is_err());
}

#[test]
fn issue_url() {
    let expected = "https://github.com/tailrocks/holla/issues/912";
    assert_eq!(issue().url(), expected);
}

#[test]
fn issue_api_path() {
    assert_eq!(issue().api_path(), "repos/tailrocks/holla/issues/912");
}

#[test]
fn issue_repo_path() {
    assert_eq!(issue().repo_path(), "repos/tailrocks/holla");
}

#[test]
fn issue_branch() {
    assert_eq!(issue().branch(), "feat/issue-912");
}

#[test]
fn project_paths() {
    let project = project();
    let url = "https://github.com/orgs/tailrocks/projects/3";
    assert_eq!(project.url(), url);
    assert_eq!(project.api_path(), "orgs/tailrocks/projectsV2/3");
}
