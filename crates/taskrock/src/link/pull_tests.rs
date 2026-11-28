//! Tests for pull request links.

use std::str::FromStr as _;

use super::PullLink;

fn pull() -> PullLink {
    PullLink {
        owner: "tailrocks".into(),
        repo: "holla".into(),
        number: 912,
    }
}

fn refuse(text: &str) -> String {
    let Err(err) = PullLink::from_str(text) else {
        panic!("{text} is refused");
    };
    err.to_string()
}

#[test]
fn both_forms_parse() {
    for text in [
        "https://github.com/tailrocks/holla/pull/912",
        "tailrocks/holla#912",
        "  tailrocks/holla#912 ",
    ] {
        assert_eq!(PullLink::from_str(text).expect("the form parses"), pull());
    }
}

#[test]
fn a_trailing_path_and_fragment_are_ignored() {
    for text in [
        "https://github.com/tailrocks/holla/pull/912/",
        "https://github.com/tailrocks/holla/pull/912/files",
        "https://github.com/tailrocks/holla/pull/912/commits",
        "https://github.com/tailrocks/holla/pull/912/checks",
        "https://github.com/tailrocks/holla/pull/912/files#diff-abc",
        "https://github.com/tailrocks/holla/pull/912#issuecomment-1",
    ] {
        let parsed = PullLink::from_str(text).expect("the tail is ignored");
        assert_eq!(parsed, pull());
    }
}

#[test]
fn anything_else_is_refused() {
    for text in [
        "",
        "https://github.com/tailrocks/holla/issues/912",
        "https://github.com/tailrocks/holla/pull/abc",
        "https://github.com/tailrocks/holla/pull/912/other",
        "https://github.com/tailrocks/holla/pull/912/files/more",
        "https://github.com/tailrocks/holla/pull",
        "holla#912",
        "tailrocks/holla#abc",
    ] {
        let expected = format!("not a link taskrock understands: not a pull request link: {text}");
        assert_eq!(refuse(text), expected);
    }
}

#[test]
fn pull_paths() {
    let pull = pull();
    assert_eq!(pull.url(), "https://github.com/tailrocks/holla/pull/912");
    assert_eq!(pull.api_path(), "repos/tailrocks/holla/pulls/912");
    assert_eq!(pull.repo_path(), "repos/tailrocks/holla");
}
