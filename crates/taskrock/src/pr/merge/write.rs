//! The writes of `pr merge`: queue or merge, and after a merge retarget the stacked pull requests.

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

/// What a retarget did: the numbers it moved, and a line for each failure.
pub(super) struct Retargeted {
    pub moved: Vec<u64>,
    pub failures: Vec<String>,
}

/// Point every open pull request that is based on the head branch at the branch the pull
/// request merged into (its base). It runs only for a merged pull request, and works whether
/// or not GitHub deleted the head branch: when it did, GitHub has retargeted the children and
/// this finds none. It tries every child and never fails: the merge landed, so a child that
/// GitHub refuses is a line in `failures`, and a rerun lists only what is left.
pub(super) fn retarget(gh: &dyn Gh, link: &PullLink, snapshot: &PullSnapshot) -> Retargeted {
    let path = format!(
        "{}/pulls?base={}&state=open&per_page=100",
        link.repo_path(),
        encode(&snapshot.head_ref)
    );
    let mut done = Retargeted {
        moved: Vec::new(),
        failures: Vec::new(),
    };
    let pulls = match get_all(gh, &path) {
        Ok(pulls) => pulls,
        Err(error) => {
            done.failures.push(format!(
                "could not list the pull requests based on {}: {}",
                snapshot.head_ref,
                message(&error)
            ));
            return done;
        }
    };
    for pull in pulls {
        let Some(number) = pull["number"].as_u64() else {
            done.failures
                .push("could not retarget a pull request that has no number".to_owned());
            continue;
        };
        let path = format!("{}/pulls/{number}", link.repo_path());
        match send(gh, "PATCH", &path, &json!({"base": snapshot.base_ref})) {
            Ok(_) => done.moved.push(number),
            Err(error) => done.failures.push(format!(
                "could not retarget #{number} to {}: {}",
                snapshot.base_ref,
                message(&error)
            )),
        }
    }
    done
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
        Err(Error::GraphQl { message }) => Ok(Land::Refused(refusal(&message))),
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
