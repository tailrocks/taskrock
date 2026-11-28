//! The writes of `pr merge`: retarget the stacked pull requests, then queue or merge.

use serde_json::json;

use crate::error::Error;
use crate::gh::Gh;
use crate::github::{get_all, graphql, is_status, send};
use crate::link::PullLink;
use crate::pr::snapshot::PullSnapshot;

const ENQUEUE: &str = "mutation($id: ID!, $head: GitObjectID!) {
  enqueuePullRequest(input: { pullRequestId: $id, expectedHeadOid: $head }) {
    mergeQueueEntry { position }
  }
}";

/// The HTTP statuses with which GitHub refuses a merge.
const REFUSED: [u16; 3] = [405, 409, 422];

/// How the pull request left our hands.
pub(super) enum Land {
    Queued(u64),
    Merged(Option<String>),
    Refused(String),
}

/// Percent-encode a value of a query string. A branch name keeps its `/`.
pub(super) fn encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

/// Point every open pull request that is based on the head branch at the default branch.
/// A rerun lists only what is left. Returns the numbers that were retargeted.
pub(super) fn retarget(
    gh: &dyn Gh,
    link: &PullLink,
    snapshot: &PullSnapshot,
) -> Result<Vec<u64>, Error> {
    let path = format!(
        "{}/pulls?base={}&state=open&per_page=100",
        link.repo_path(),
        encode(&snapshot.head_ref)
    );
    let mut moved = Vec::new();
    for pull in get_all(gh, &path)? {
        let number = pull["number"]
            .as_u64()
            .ok_or_else(|| Error::Json("pulls: a pull request has no number".to_owned()))?;
        let path = format!("{}/pulls/{number}", link.repo_path());
        send(
            gh,
            "PATCH",
            &path,
            &json!({"base": snapshot.default_branch}),
        )?;
        moved.push(number);
    }
    Ok(moved)
}

/// Queue the pull request when its base has a merge queue, otherwise squash-merge it.
/// Both writes name the head that was judged, so a push in between makes GitHub refuse.
pub(super) fn land(gh: &dyn Gh, link: &PullLink, snapshot: &PullSnapshot) -> Result<Land, Error> {
    if snapshot.queue.has_queue {
        enqueue(gh, snapshot)
    } else {
        squash(gh, link, snapshot)
    }
}

fn enqueue(gh: &dyn Gh, snapshot: &PullSnapshot) -> Result<Land, Error> {
    let variables = json!({"id": snapshot.node_id, "head": snapshot.head_sha});
    match graphql(gh, ENQUEUE, &variables) {
        Ok(data) => Ok(data["enqueuePullRequest"]["mergeQueueEntry"]["position"]
            .as_u64()
            .map_or_else(
                || Land::Refused(refusal("GitHub returned no merge queue entry")),
                Land::Queued,
            )),
        Err(error @ Error::Gh { .. }) => Ok(Land::Refused(refusal(&message(&error)))),
        Err(error) => Err(error),
    }
}

fn squash(gh: &dyn Gh, link: &PullLink, snapshot: &PullSnapshot) -> Result<Land, Error> {
    let path = format!("{}/merge", link.api_path());
    let body = json!({"merge_method": "squash", "sha": snapshot.head_sha});
    match send(gh, "PUT", &path, &body) {
        Ok(answer) => Ok(Land::Merged(answer["sha"].as_str().map(str::to_owned))),
        Err(error) if REFUSED.iter().any(|code| is_status(&error, *code)) => {
            Ok(Land::Refused(refusal(&message(&error))))
        }
        Err(error) => Err(error),
    }
}

fn refusal(message: &str) -> String {
    format!("GitHub refused the merge: {message}")
}

/// What GitHub said, without the `gh: ` prefix.
fn message(error: &Error) -> String {
    match error {
        Error::Gh { stderr, .. } => stderr.strip_prefix("gh: ").unwrap_or(stderr).to_owned(),
        other => other.to_string(),
    }
}
