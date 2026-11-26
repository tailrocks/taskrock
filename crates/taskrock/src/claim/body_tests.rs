//! Tests of the body check of `claim`.

use crate::error::Error;

use super::claim;
use crate::gh::FakeGh;

use super::tests::{FIELDS, LOOKUP, TASK, URL, full, has, pages};

const NO_CONFIG: &str = "gh: Not Found (HTTP 404)";

pub(super) fn body(sections: &[&str]) -> String {
    let text = sections.iter().fold(String::new(), |all, name| {
        all + &format!("## {name}\n\nx\n\n")
    });
    let text = format!("{text}## Done when\n\n- [ ] `cargo test` passes\n");
    serde_json::json!({ "body": text }).to_string()
}

pub(super) fn task_body() -> String {
    body(&["Outcome", "Decisions already made", "Steps", "Out of scope"])
}

pub(super) fn with_body(items: &str, issue: &str) -> FakeGh {
    FakeGh::new()
        .ok("graphql", LOOKUP)
        .ok("fields", &format!("[{FIELDS}]"))
        .ok("items", items)
        .ok("issues/7", issue)
        .fail("contents/taskrock.toml", NO_CONFIG)
        .ok("repos/o/r", r#"{"default_branch":"main"}"#)
        .ok("ref/heads", r#"{"object":{"sha":"abc"}}"#)
}

#[test]
fn good_body_is_taken() {
    let gh = full(TASK, None);
    claim(&gh, URL).expect("claim succeeds");
    assert!(has(&gh, "POST"));
}

#[test]
fn bug_without_cause_is_refused_before_any_branch_call() {
    let bug = TASK.replace("Task", "Bug");
    let sections = [
        "What happens",
        "Expected",
        "How to reproduce",
        "Environment",
    ];
    let sections = [
        &sections[..],
        &["Decisions already made", "Steps", "Out of scope"],
    ]
    .concat();
    let gh = with_body(&pages(&bug, None), &body(&sections)).ok("POST", "{}");
    let error = claim(&gh, URL).expect_err("the body is incomplete");
    let message = "the body breaks the Bug format: the section ## Cause is missing";
    assert!(matches!(&error, Error::Refused(got) if got == message));
    assert!(!has(&gh, "git/refs"));
    assert!(!has(&gh, "git/ref/heads"));
}

#[test]
fn feature_with_a_full_body_is_taken() {
    let feature = TASK.replace("Task", "Feature");
    let sections = ["Problem", "Proposed behavior", "Workaround today"];
    let sections = [
        &sections[..],
        &["Decisions already made", "Steps", "Out of scope"],
    ]
    .concat();
    let gh = with_body(&pages(&feature, None), &body(&sections))
        .ok("POST", "{}")
        .ok("fields", &format!("[{FIELDS}]"))
        .ok("PATCH", "")
        .ok("PATCH", "");
    claim(&gh, URL).expect("claim succeeds");
    assert!(has(&gh, "POST"));
}
