//! Tests of `parent` and `blocked_by` that are links to existing issues.

use super::tests::{issue, lines};

const LINK: &str = "https://github.com/tailrocks/taskrock/issues/50";

#[test]
fn links_are_accepted_as_parent_and_blocker() {
    let found = lines(vec![issue(
        "T1",
        &format!("parent = \"{LINK}\"\nblocked_by = [\"{LINK}\"]"),
    )]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn bad_link_is_reported_with_its_shape() {
    let bad = "https://github.com/tailrocks/taskrock/pull/50";
    let found = lines(vec![issue(
        "T1",
        &format!("parent = \"{bad}\"\nblocked_by = [\"{bad}\"]"),
    )]);
    let shape = "is not an issue link like https://github.com/<owner>/<repo>/issues/<number>";
    assert_eq!(
        found,
        [
            format!("T1.md: blocker {bad} {shape}"),
            format!("T1.md: parent {bad} {shape}"),
        ]
    );
}

#[test]
fn a_missing_key_is_still_reported() {
    assert_eq!(
        lines(vec![issue("T1", "parent = \"E9\"")]),
        ["T1.md: parent E9 is not a key in this draft"]
    );
}

#[test]
fn http_link_gets_the_link_shape_message() {
    let bad = "http://github.com/tailrocks/taskrock/issues/50";
    let shape = "is not an issue link like https://github.com/<owner>/<repo>/issues/<number>";
    assert_eq!(
        lines(vec![issue("T1", &format!("parent = \"{bad}\""))]),
        [format!("T1.md: parent {bad} {shape}")]
    );
}
