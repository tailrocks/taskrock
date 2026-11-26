//! Tests of the links in a draft.

use super::{is_link, parse};

#[test]
fn full_issue_link_parses() {
    let link = parse("https://github.com/tailrocks/taskrock/issues/50").expect("a link");
    assert_eq!(
        (link.owner.as_str(), link.repo.as_str(), link.number),
        ("tailrocks", "taskrock", 50)
    );
}

#[test]
fn only_the_full_shape_is_a_link() {
    for bad in [
        "https://github.com/tailrocks/taskrock/pull/50",
        "https://github.com/tailrocks/taskrock/issues/50/",
        "https://github.com/tailrocks/taskrock/issues/50#x",
        "https://example.com/tailrocks/taskrock/issues/50",
        "https://api.github.com/repos/tailrocks/taskrock/issues/50",
        "tailrocks/taskrock#50",
        "E1",
    ] {
        assert!(parse(bad).is_none(), "{bad}");
    }
}

#[test]
fn a_value_that_starts_with_https_is_meant_as_a_link() {
    assert!(is_link("https://example.com"));
    assert!(!is_link("E1"));
}
