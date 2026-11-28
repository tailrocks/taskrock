//! Tests of `pr threads` against a scripted `gh`.

use serde_json::{Value, json};

use super::{Action, Report, render};
use crate::gh::FakeGh;
use crate::link::PullLink;

mod defer;
mod resolve;

pub(super) const HEAD: &str = "0123456789abcdef0123456789abcdef01234567";
pub(super) const P1: &str = "![P1 Badge](https://img.shields.io/badge/P1-red)\nfix it";
pub(super) const P3: &str = "![P3 Badge](https://img.shields.io/badge/P3-green)\nnit";

pub(super) fn link() -> PullLink {
    PullLink {
        owner: "tailrocks".into(),
        repo: "taskrock".into(),
        number: 5,
    }
}

pub(super) fn resolve(id: &str, reason: &str) -> Action {
    Action::Resolve {
        id: id.into(),
        reason: reason.into(),
    }
}

/// A review thread whose first comment has the database id `db`.
pub(super) fn thread(
    id: &str,
    db: u64,
    who: (&str, &str),
    bodies: &[&str],
    resolved: bool,
) -> Value {
    let nodes: Vec<Value> = bodies
        .iter()
        .map(|body| {
            let url = format!("https://github.com/tailrocks/taskrock/pull/5#discussion_r{db}");
            json!({"databaseId": db, "url": url, "body": body,
                "author": {"login": who.0, "__typename": who.1}})
        })
        .collect();
    json!({"id": id, "isResolved": resolved,
        "comments": {"totalCount": nodes.len(), "nodes": nodes}})
}

pub(super) fn bot(id: &str, db: u64, body: &str) -> Value {
    thread(id, db, ("review-bot[bot]", "Bot"), &[body], false)
}

pub(super) fn pull(state: &str, sha: &str) -> String {
    let repo = json!({"full_name": "tailrocks/taskrock", "default_branch": "main"});
    json!({"number": 5, "node_id": "PR_5", "state": state, "merged": false, "body": "",
        "base": {"ref": "main", "repo": repo}, "head": {"ref": "feat/issue-5", "sha": sha, "repo": repo}})
    .to_string()
}

/// The reads of one snapshot of the pull request, with `threads` as its review threads.
pub(super) fn reads(threads: Vec<Value>) -> FakeGh {
    let graph = json!({"data": {"repository": {"mergeQueue": null, "pullRequest": {
        "mergeQueueEntry": null,
        "reviewThreads": {"pageInfo": {"hasNextPage": false, "endCursor": null}, "nodes": threads}}}}});
    FakeGh::new()
        .fail("contents/taskrock.toml", "gh: Not Found (HTTP 404)")
        .ok("pulls/5/reviews", "[[]]")
        .ok("issues/5/comments", "[[]]")
        .ok("check-runs", r#"{"total_count":0,"check_runs":[]}"#)
        .ok("/status", r#"{"total_count":0,"statuses":[]}"#)
        .ok("rules/branches", "[[]]")
        .ok("graphql", &graph.to_string())
        .ok("pulls/5", &pull("open", HEAD))
}

/// The answer of GitHub to `resolveReviewThread`.
pub(super) fn resolved() -> String {
    json!({"data": {"resolveReviewThread": {"thread": {"isResolved": true}}}}).to_string()
}

/// The writes a run made, in order: `reply <path> <body>` and `resolve <thread id>`.
pub(super) fn writes(gh: &FakeGh) -> Vec<String> {
    gh.calls()
        .into_iter()
        .filter_map(|(args, input)| {
            let input: Value = serde_json::from_str(&input?).ok()?;
            if args.contains("-X POST") {
                let path = args.split_whitespace().nth(3)?;
                Some(format!("reply {path} {}", input["body"].as_str()?))
            } else {
                let id = input["variables"]["id"].as_str()?;
                Some(format!("resolve {id}"))
            }
        })
        .collect()
}

#[test]
fn threads_render_text_names_each_thread_and_the_failure() {
    let report = Report {
        schema: 1,
        resolved: vec!["u1".into()],
        failed: Some("boom".into()),
    };
    assert_eq!(
        render(&report, 2, false).expect("text"),
        "resolved u1\nfailed: boom\nresolved 1 of 2 thread(s)"
    );
    let json: Value = serde_json::from_str(&render(&report, 2, true).expect("json")).expect("JSON");
    assert_eq!(
        json,
        json!({"schema": 1, "resolved": ["u1"], "failed": "boom"})
    );
}
