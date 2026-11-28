//! The recorded replies of one pull request, for the tests of the snapshot.

use std::path::PathBuf;

use serde_json::json;

use crate::gh::FakeGh;
use crate::link::PullLink;

/// The replies of one pull request of tailrocks/velnor-new, in the order of `read`.
pub(crate) struct Case {
    pub(crate) number: u64,
    pub pull: String,
    pub checks: String,
    pub status: String,
    pub rules: Result<String, String>,
    pub comments: String,
    pub graphs: Vec<String>,
}

fn fixture(part: &str, name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/pr")
        .join(format!("{part}-{name}.json"));
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

impl Case {
    /// The fixtures of pull request `number`, with the rules of `main`.
    pub(crate) fn recorded(number: u64) -> Self {
        let n = number.to_string();
        Self {
            number,
            pull: fixture("pull", &n),
            checks: fixture("checks", &n),
            status: fixture("status", &n),
            rules: Ok(fixture("rules", "main")),
            comments: fixture("comments", &n),
            graphs: vec![fixture("threads", &n)],
        }
    }

    pub(crate) fn link(&self) -> PullLink {
        PullLink {
            owner: "tailrocks".into(),
            repo: "velnor-new".into(),
            number: self.number,
        }
    }

    /// A `FakeGh` with these replies. A longer path comes before a shorter one.
    pub(crate) fn gh(&self) -> FakeGh {
        let n = self.number;
        let gh = FakeGh::new()
            .ok(
                &format!("pulls/{n}/reviews"),
                &fixture("reviews", &n.to_string()),
            )
            .ok(&format!("issues/{n}/comments"), &self.comments)
            .ok("check-runs", &self.checks)
            .ok("/status", &self.status);
        let gh = match &self.rules {
            Ok(rules) => gh.ok("rules/branches", rules),
            Err(stderr) => gh.fail("rules/branches", stderr),
        };
        let gh = self
            .graphs
            .iter()
            .fold(gh, |gh, graph| gh.ok("graphql", graph));
        gh.ok(&format!("pulls/{n}"), &self.pull)
    }
}

/// One page of the GraphQL answer with `threads` resolved threads.
pub(crate) fn page(threads: usize, next: Option<&str>, comments: u64) -> String {
    let node = json!({"id": "T", "isResolved": true, "comments": {"totalCount": comments, "nodes": [
        {"databaseId": 7, "url": "u", "body": "b", "author": {"login": "bot", "__typename": "Bot"}}]}});
    let info = json!({"hasNextPage": next.is_some(), "endCursor": next});
    json!({"data": {"repository": {"mergeQueue": null, "pullRequest": {"mergeQueueEntry": null,
        "reviewThreads": {"pageInfo": info, "nodes": vec![node; threads]}}}}})
    .to_string()
}
