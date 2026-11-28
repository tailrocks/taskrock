//! Tests of `pr merge` against a scripted `gh`.

use serde_json::{Value, json};

use super::{Done, execute};
use crate::error::Error;
use crate::gh::FakeGh;
use crate::link::PullLink;

mod land;
mod stop;

pub(super) const HEAD: &str = "0123456789abcdef0123456789abcdef01234567";
pub(super) const OLD: &str = "fedcba9876543210fedcba9876543210fedcba98";

pub(super) fn link() -> PullLink {
    PullLink {
        owner: "tailrocks".into(),
        repo: "taskrock".into(),
        number: 5,
    }
}

/// What the pull request is.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Open,
    Merged,
    Fork,
}

/// What the reads of the pull request find. `ready()` finds nothing to wait for.
#[derive(Clone)]
pub(super) struct Scene {
    pub queue: bool,
    pub kind: Kind,
    pub passed: bool,
    pub codex_head: Option<&'static str>,
    /// A check run on the head. Without one, CI has not started.
    pub check_started: bool,
}

impl Scene {
    pub(super) fn ready() -> Self {
        Self {
            queue: false,
            kind: Kind::Open,
            passed: true,
            codex_head: None,
            check_started: true,
        }
    }

    fn pull(&self) -> String {
        let repo = |name: &str| json!({"full_name": name, "default_branch": "main"});
        let merged = self.kind == Kind::Merged;
        let head = if self.kind == Kind::Fork {
            "someone/taskrock"
        } else {
            "tailrocks/taskrock"
        };
        json!({"number": 5, "node_id": "PR_5", "merged": merged,
            "state": if merged { "closed" } else { "open" },
            "merge_commit_sha": "c0ffee0123456789c0ffee0123456789c0ffee01",
            "body": "Closes #5", "mergeable": true, "mergeable_state": "clean",
            "base": {"ref": "main", "repo": repo("tailrocks/taskrock")},
            "head": {"ref": "feat/issue-5", "sha": HEAD, "repo": repo(head)}})
        .to_string()
    }

    fn reviews(&self) -> String {
        let body = format!("VERDICT: PASS head={HEAD}");
        let notes = json!([{"body": body, "submitted_at": "2026-11-28T10:00:00Z", "user": {"login": "reviewer"}}]);
        if self.passed {
            json!([notes])
        } else {
            json!([[]])
        }
        .to_string()
    }

    fn comments(&self) -> String {
        let note = |sha: &str| {
            let state = json!({"code_review": {"commit_sha": sha, "status": "completed"},
                "security_review": {"commit_sha": sha, "status": "completed"}});
            json!({"body": format!("<!-- codex-review-state:v1 {state} -->"),
                "created_at": "2026-11-28T10:00:00Z", "user": {"login": "chatgpt-codex-connector[bot]"}})
        };
        json!([self.codex_head.map(note).into_iter().collect::<Vec<_>>()]).to_string()
    }

    fn check_runs(&self) -> String {
        let runs = if self.check_started {
            json!([{"name": "Required", "status": "completed", "conclusion": "success"}])
        } else {
            json!([])
        };
        json!({"total_count": runs.as_array().map_or(0, Vec::len), "check_runs": runs}).to_string()
    }

    /// Every read of one snapshot, then no reply for any write.
    pub(super) fn gh(&self) -> FakeGh {
        let queue = self.queue.then(|| json!({"id": "MQ"}));
        let graph = json!({"data": {"repository": {"mergeQueue": queue, "pullRequest": {
            "mergeQueueEntry": null, "reviewThreads": {
                "pageInfo": {"hasNextPage": false, "endCursor": null}, "nodes": []}}}}});
        FakeGh::new()
            .fail("contents/taskrock.toml", "gh: Not Found (HTTP 404)")
            .ok("pulls/5/reviews", &self.reviews())
            .ok("issues/5/comments", &self.comments())
            .ok("check-runs", &self.check_runs())
            .ok("/status", r#"{"total_count":0,"statuses":[]}"#)
            .ok("rules/branches", "[[]]")
            .ok("pulls/5", &self.pull())
            .ok("graphql", &graph.to_string())
    }
}

/// The writes a run made, in order: `<METHOD> <path> <body>`, or `graphql <variables>`.
pub(super) fn writes(gh: &FakeGh) -> Vec<String> {
    gh.calls()
        .into_iter()
        .filter_map(|(args, input)| {
            let input = input?;
            if args.contains(" -X ") {
                let mut words = args.split_whitespace().skip(2);
                Some(format!("{} {} {input}", words.next()?, words.next()?))
            } else if input.contains("mutation") {
                let input: Value = serde_json::from_str(&input).ok()?;
                Some(format!("graphql {}", input["variables"]))
            } else {
                None
            }
        })
        .collect()
}

pub(super) fn run(gh: &FakeGh) -> Result<Done, Error> {
    execute(gh, &link())
}
