//! Tests of the blocker links.

use serde_json::{Value, json};

use crate::apply::blockers::link_blockers;
use crate::apply::create_issues;
use crate::gh::FakeGh;

use super::super::tests::{
    all_existing, base, dry_options, folder, fresh, has_call, not_found, options,
};

/// The inputs of the blocker `POST` calls, in call order.
fn blocker_posts(gh: &FakeGh) -> Vec<Value> {
    gh.calls()
        .into_iter()
        .filter(|(call, _)| call.contains("POST") && call.contains("blocked_by"))
        .map(|(_, input)| {
            serde_json::from_str(&input.expect("a POST sends a body")).expect("input is JSON")
        })
        .collect()
}

/// Runs the creation and then the blockers on `gh` and returns the lines
/// the blockers print.
fn linked(gh: &FakeGh, dry_run: bool) -> Vec<String> {
    let (draft, found, _) = if dry_run {
        create_issues(gh, &folder(), &dry_options()).expect("dry run succeeds")
    } else {
        create_issues(gh, &folder(), &options()).expect("apply succeeds")
    };
    link_blockers(gh, &draft, &found, dry_run).expect("blockers succeed")
}

#[test]
fn existing_blocker_is_not_sent_again() {
    let gh = all_existing().ok(
        "issues/24/dependencies/blocked_by?per_page",
        r#"[[{"id":203}]]"#,
    );
    let lines = linked(&gh, false);
    assert!(lines.is_empty(), "nothing is linked: {lines:?}");
    assert!(
        blocker_posts(&gh).is_empty(),
        "the known blocker is not sent"
    );
}

#[test]
fn missing_blocker_is_sent_with_blocker_id() {
    let gh = fresh()
        .ok("issues/14/dependencies/blocked_by?per_page", "[[]]")
        .ok(
            "POST repos/tailrocks/holla/issues/14/dependencies/blocked_by",
            "{}",
        );
    let lines = linked(&gh, false);
    assert_eq!(lines, ["blocked #14 by #13"]);
    assert_eq!(blocker_posts(&gh), [json!({"issue_id": 113})]);
}

#[test]
fn dry_run_sends_no_post() {
    let gh = not_found(base().ok("issues?state=all", "[[]]"), 1);
    let lines = linked(&gh, true);
    assert!(!has_call(&gh, "POST"), "a dry run sends no POST");
    assert_eq!(lines, ["would block T02 by T01"]);
}
