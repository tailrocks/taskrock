//! Tests of the root rule: only an Initiative has no `parent`.

use super::tests::{epic, issue, lines};
use crate::draft::DraftIssue;

const NO_PARENT: &str = "has no parent. Only an Initiative has none: set parent to a key of this draft or to the link of the parent issue";

fn root(key: &str, kind: &str) -> DraftIssue {
    let mut full = if kind == "Epic" {
        epic(key)
    } else {
        issue(key, "")
    };
    kind.clone_into(&mut full.kind);
    full.parent = None;
    full
}

fn parentless(found: Vec<String>) -> Vec<String> {
    found
        .into_iter()
        .filter(|line| line.contains(NO_PARENT))
        .collect()
}

#[test]
fn a_root_that_is_not_an_initiative_is_refused() {
    for kind in ["Task", "Bug", "Feature", "Epic"] {
        let found = parentless(lines(vec![root("R", kind)]));
        assert_eq!(found, [format!("R.md: R {NO_PARENT}")], "{kind}");
    }
}

#[test]
fn an_initiative_root_is_accepted() {
    assert_eq!(parentless(lines(vec![root("I", "Initiative")])).len(), 0);
}

#[test]
fn a_key_parent_and_a_link_parent_are_accepted() {
    let link = "parent = \"https://github.com/tailrocks/taskrock/issues/50\"";
    assert_eq!(
        lines(vec![epic("E"), issue("T", "parent = \"E\"")]).len(),
        0
    );
    assert_eq!(lines(vec![issue("T", link)]).len(), 0);
}

#[test]
fn a_bare_number_names_the_link_to_write() {
    let found = lines(vec![issue(
        "T",
        "parent = \"2878\"\nblocked_by = [\"#2878\"]",
    )]);
    let link = "is an issue number, not a key: write the issue link https://github.com/tailrocks/holla/issues/2878";
    assert_eq!(
        found,
        [
            format!("T.md: blocker #2878 {link}"),
            format!("T.md: parent 2878 {link}"),
        ]
    );
}

#[test]
fn the_link_uses_the_repository_of_the_issue() {
    let found = lines(vec![issue("T", "repo = \"o/other\"\nparent = \"7\"")]);
    assert_eq!(
        found,
        [
            "T.md: parent 7 is an issue number, not a key: write the issue link https://github.com/o/other/issues/7"
        ]
    );
}

#[test]
fn a_key_that_is_all_digits_is_still_accepted() {
    let found = lines(vec![epic("2878"), issue("T", "parent = \"2878\"")]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn an_empty_parent_is_not_called_an_issue_number() {
    for parent in ["", "#"] {
        let found = lines(vec![issue("T", &format!("parent = \"{parent}\""))]);
        assert_eq!(found.len(), 1, "{parent:?}: {found:?}");
        assert!(!found[0].contains("is an issue number"), "{found:?}");
    }
}
