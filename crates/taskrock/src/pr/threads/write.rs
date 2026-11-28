//! The two writes that settle a review thread: a reply, then a resolve.

use serde_json::json;

use crate::error::Error;
use crate::gh::Gh;
use crate::github::{graphql, send};
use crate::link::PullLink;
use crate::pr::snapshot::ThreadFact;

const RESOLVE: &str = "mutation($id: ID!) {
  resolveReviewThread(input: { threadId: $id }) { thread { isResolved } }
}";

/// The hidden line that ends every reply of taskrock and names its thread.
fn marker(thread: &ThreadFact) -> String {
    format!("<!-- taskrock:{} -->", thread.id)
}

/// Reply to `thread` with `text`, unless a reply of taskrock is already there,
/// then resolve it. A rerun after a crash between the two writes only resolves.
pub(super) fn settle(
    gh: &dyn Gh,
    link: &PullLink,
    thread: &ThreadFact,
    text: &str,
) -> Result<(), Error> {
    let marker = marker(thread);
    if !thread.bodies.iter().any(|body| body.contains(&marker)) {
        reply(gh, link, thread, &format!("{text}\n\n{marker}"))?;
    }
    resolve(gh, thread)
}

/// REST `POST pulls/<n>/comments/<first comment>/replies`.
fn reply(gh: &dyn Gh, link: &PullLink, thread: &ThreadFact, body: &str) -> Result<(), Error> {
    let path = format!(
        "{}/pulls/{}/comments/{}/replies",
        link.repo_path(),
        link.number,
        thread.first_comment_id
    );
    send(gh, "POST", &path, &json!({ "body": body })).map(drop)
}

/// GraphQL `resolveReviewThread`. It succeeds only when GitHub says the thread is resolved.
fn resolve(gh: &dyn Gh, thread: &ThreadFact) -> Result<(), Error> {
    let data = graphql(gh, RESOLVE, &json!({ "id": thread.id }))?;
    let resolved = data["resolveReviewThread"]["thread"]["isResolved"].as_bool();
    if resolved == Some(true) {
        Ok(())
    } else {
        Err(Error::Gh {
            args: "api graphql resolveReviewThread".to_owned(),
            stderr: format!("GitHub did not resolve the thread {}", thread.url),
        })
    }
}
