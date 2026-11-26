//! Tests of `claim_stacked`.

use crate::error::Error;
use crate::gh::FakeGh;

use super::super::body_tests::{task_body, with_body};
use super::super::tests::{FIELDS, LOOKUP, TASK, URL, has, pages};
use super::claim_stacked;

const OPEN: &str = r#""issue_dependencies_summary":{"blocked_by":1,"blocking":0}"#;
const FREE: &str = r#""issue_dependencies_summary":{"blocked_by":0,"blocking":0}"#;
const BLOCKER: &str = r#"[[{"state":"open","html_url":"https://github.com/o/r/issues/5"}]]"#;
const PULL: &str = r#"[[{"html_url":"https://github.com/o/r/pull/105","base":{"ref":"main"}}]]"#;

fn blocker_item() -> String {
    TASK.replace("issues/7", "issues/5")
        .replace("\"id\":7", "\"id\":5")
        .replace("Todo", "In Progress")
        .replace(
            ", \"value\":{\"url\":\"https://github.com/o/r/issues/5\"}",
            "",
        )
        .replace("{\"url\":\"https://github.com/o/r/issues/5\"}", "null")
}

/// A fake for a stacked claim: the replies up to the branch are in call order.
fn until_pull_request() -> FakeGh {
    let task = TASK.replace(FREE, OPEN);
    FakeGh::new()
        .ok("graphql", LOOKUP)
        .ok("fields", &format!("[{FIELDS}]"))
        .ok("items", &format!("[[{task},{}]]", blocker_item()))
        .ok("blocked_by", BLOCKER)
        .ok("head=o:feat/issue-5&", PULL)
}

fn stacked_gh() -> FakeGh {
    until_pull_request()
        .ok("issues/7", &task_body())
        .fail("contents/taskrock.toml", "gh: Not Found (HTTP 404)")
        .ok("ref/heads/feat/issue-5", r#"{"object":{"sha":"def"}}"#)
        .ok("git/refs", "{}")
        .ok("comments", r#"{"id":42}"#)
        .ok("fields", &format!("[{FIELDS}]"))
        .ok("PATCH", "")
        .ok("PATCH", "")
}

fn posts(gh: &FakeGh) -> Vec<(String, String)> {
    gh.calls()
        .into_iter()
        .filter(|(call, _)| call.contains("-X POST"))
        .map(|(call, input)| (call, input.unwrap_or_default()))
        .collect()
}

#[test]
fn stacked_claim_starts_from_the_blocker_branch() {
    let gh = stacked_gh();
    let out = claim_stacked(&gh, URL).expect("claim succeeds");
    assert_eq!(
        out,
        "taken https://github.com/o/r/issues/7\nbranch feat/issue-7\nbase feat/issue-5"
    );
    let posts = posts(&gh);
    let (_, branch) = posts
        .iter()
        .find(|(call, _)| call.contains("git/refs"))
        .expect("the branch is created");
    assert!(branch.contains(r#""sha":"def""#), "{branch}");
    let (_, comment) = posts
        .iter()
        .find(|(call, _)| call.contains("issues/7/comments"))
        .expect("the comment is posted");
    assert!(comment.contains("<!-- taskrock:stacked-on feat/issue-5 -->"));
}

#[test]
fn stacked_claim_without_blockers_starts_from_the_default_branch() {
    let gh = with_body(&pages(TASK, None), &task_body())
        .ok("POST", "{}")
        .ok("fields", &format!("[{FIELDS}]"))
        .ok("PATCH", "")
        .ok("PATCH", "");
    let out = claim_stacked(&gh, URL).expect("claim succeeds");
    assert_eq!(
        out,
        "taken https://github.com/o/r/issues/7\nbranch feat/issue-7\nbase main"
    );
    assert!(!has(&gh, "comments"));
}

#[test]
fn refused_stack_writes_nothing() {
    let task = TASK.replace(FREE, OPEN);
    let gh = FakeGh::new()
        .ok("graphql", LOOKUP)
        .ok("fields", &format!("[{FIELDS}]"))
        .ok("items", &format!("[[{task},{}]]", blocker_item()))
        .ok("blocked_by", BLOCKER)
        .ok("head=o:feat/issue-5&", "[[]]");
    let error = claim_stacked(&gh, URL).expect_err("the stack is refused");
    assert!(matches!(error, Error::Refused(_)));
    assert!(!has(&gh, "-X POST"));
}

#[test]
fn failed_status_deletes_the_branch_and_the_comment() {
    let gh = until_pull_request()
        .ok("issues/7", &task_body())
        .fail("contents/taskrock.toml", "gh: Not Found (HTTP 404)")
        .ok("ref/heads/feat/issue-5", r#"{"object":{"sha":"def"}}"#)
        .ok("git/refs", "{}")
        .ok("comments", r#"{"id":42}"#)
        .fail("fields", "gh: boom (HTTP 500)")
        .ok("DELETE", "")
        .ok("DELETE", "");
    claim_stacked(&gh, URL).expect_err("the status fails");
    assert!(has(&gh, "DELETE repos/o/r/git/refs/heads/feat/issue-7"));
    assert!(has(&gh, "DELETE repos/o/r/issues/comments/42"));
}
