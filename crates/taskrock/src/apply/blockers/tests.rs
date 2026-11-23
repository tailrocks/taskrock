//! Tests of the blocker links.

use serde_json::{Value, json};

use crate::apply::blockers::link_blockers;
use crate::apply::create_issues;
use crate::gh::FakeGh;

use super::super::tests::{base, created, dry_options, folder, has_call, not_found, options};

/// The three issues of `draft-good`, with numbers 21 to 23.
fn existing() -> FakeGh {
    not_found(
        base().ok(
            "issues?state=all",
            r#"[[{"number":21,"id":201,"body":"a\n\n<!-- taskrock:holla-csv:E1 -->"},{"number":22,"id":202,"body":"b\n\n<!-- taskrock:holla-csv:T01 -->"},{"number":23,"id":203,"body":"c\n\n<!-- taskrock:holla-csv:T02 -->"}]]"#,
        ),
        24,
    )
}

/// The three issues of `draft-good` created with numbers 11 to 13.
fn missing() -> FakeGh {
    created(
        not_found(
            base().ok(
                "issues?state=all",
                r#"[[{"number":7,"id":70,"body":"old"},{"number":8,"id":80,"pull_request":{},"body":"old"}]]"#,
            ),
            9,
        ),
        11,
    )
}

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

#[test]
fn existing_blocker_is_not_sent_again() {
    let gh = existing().ok(
        "issues/23/dependencies/blocked_by?per_page",
        r#"[[{"id":202}]]"#,
    );
    let (draft, found, _) = create_issues(&gh, &folder(), &options()).expect("apply succeeds");
    let lines = link_blockers(&gh, &draft, &found, false).expect("blockers succeed");
    assert!(lines.is_empty(), "nothing is linked: {lines:?}");
    assert!(
        blocker_posts(&gh).is_empty(),
        "the known blocker is not sent"
    );
}

#[test]
fn missing_blocker_is_sent_with_blocker_id() {
    let gh = missing()
        .ok("issues/13/dependencies/blocked_by?per_page", "[[]]")
        .ok(
            "POST repos/tailrocks/holla/issues/13/dependencies/blocked_by",
            "{}",
        );
    let (draft, found, _) = create_issues(&gh, &folder(), &options()).expect("apply succeeds");
    let lines = link_blockers(&gh, &draft, &found, false).expect("blockers succeed");
    assert_eq!(lines, ["blocked #13 by #12"]);
    assert_eq!(blocker_posts(&gh), [json!({"issue_id": 112})]);
}

#[test]
fn dry_run_sends_no_post() {
    let gh = not_found(base().ok("issues?state=all", "[[]]"), 1);
    let (draft, found, _) =
        create_issues(&gh, &folder(), &dry_options()).expect("dry run succeeds");
    let lines = link_blockers(&gh, &draft, &found, true).expect("dry run succeeds");
    assert!(!has_call(&gh, "POST"), "a dry run sends no POST");
    assert_eq!(lines, ["would block T02 by T01"]);
}
