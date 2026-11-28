//! The one GraphQL query of a snapshot: review threads and the merge queue.

use serde::Deserialize;
use serde_json::json;

use super::{QueueEntry, ThreadFact};
use crate::error::Error;
use crate::gh::Gh;
use crate::github::graphql;
use crate::link::PullLink;

/// How many pages of 100 threads are read.
const MAX_PAGES: usize = 10;
const PAGE: u64 = 100;

const QUERY: &str =
    "query($owner: String!, $name: String!, $n: Int!, $base: String!, $cursor: String) {
  repository(owner: $owner, name: $name) {
    mergeQueue(branch: $base) { id }
    pullRequest(number: $n) {
      mergeQueueEntry { state position }
      reviewThreads(first: 100, after: $cursor) {
        pageInfo { hasNextPage endCursor }
        nodes {
          id isResolved
          comments(first: 100) {
            totalCount
            nodes { databaseId url body author { login __typename } }
          }
        }
      }
    }
  }
}";

/// What the query found.
#[derive(Debug, Default)]
pub(super) struct Graph {
    pub threads: Vec<ThreadFact>,
    pub has_queue: bool,
    pub entry: Option<QueueEntry>,
}

#[derive(Deserialize)]
struct Data {
    repository: Repository,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Repository {
    merge_queue: Option<serde_json::Value>,
    pull_request: Pull,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pull {
    merge_queue_entry: Option<Entry>,
    review_threads: Threads,
}

#[derive(Deserialize)]
struct Entry {
    state: String,
    position: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Threads {
    page_info: PageInfo,
    nodes: Vec<Thread>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_next_page: bool,
    end_cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Thread {
    id: String,
    is_resolved: bool,
    comments: Comments,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Comments {
    total_count: u64,
    nodes: Vec<Comment>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Comment {
    database_id: u64,
    url: String,
    body: String,
    author: Option<Author>,
}

#[derive(Deserialize)]
struct Author {
    login: String,
    #[serde(rename = "__typename")]
    typename: String,
}

/// Read the threads of the pull request, page by page, and its place in the merge queue.
pub(super) fn read(
    gh: &dyn Gh,
    link: &PullLink,
    base: &str,
    problems: &mut Vec<String>,
) -> Result<Graph, Error> {
    let mut graph = Graph::default();
    let mut cursor: Option<String> = None;
    for page in 0..MAX_PAGES {
        let data = match fetch(gh, link, base, cursor.as_deref()) {
            Ok(data) => data,
            Err(error) if page > 0 => {
                problems.push(format!(
                    "The review threads of the later pages could not be read, so the snapshot holds only the {} threads of the earlier pages: {error}.",
                    graph.threads.len()
                ));
                return Ok(graph);
            }
            Err(error) => return Err(error),
        };
        let pull = data.repository.pull_request;
        if page == 0 {
            graph.has_queue = data.repository.merge_queue.is_some();
            graph.entry = pull.merge_queue_entry.map(|e| QueueEntry {
                state: e.state,
                position: e.position,
            });
        }
        for thread in pull.review_threads.nodes {
            graph.threads.extend(fact(thread, problems));
        }
        let info = pull.review_threads.page_info;
        match info.end_cursor.filter(|_| info.has_next_page) {
            Some(next) => cursor = Some(next),
            None => return Ok(graph),
        }
    }
    problems.push(format!(
        "The pull request has more than {MAX_PAGES} pages of review threads and only {} were read.",
        graph.threads.len()
    ));
    Ok(graph)
}

fn fact(thread: Thread, problems: &mut Vec<String>) -> Option<ThreadFact> {
    if thread.comments.total_count > PAGE {
        problems.push(format!(
            "The review thread {} has {} comments and only {PAGE} were read.",
            thread.id, thread.comments.total_count
        ));
    }
    let Some(first) = thread.comments.nodes.first() else {
        problems.push(format!("The review thread {} has no comments.", thread.id));
        return None;
    };
    let (first_author, first_is_bot) = first.author.as_ref().map_or_else(
        || ("ghost".to_owned(), false),
        |a| (a.login.clone(), a.typename == "Bot"),
    );
    Some(ThreadFact {
        id: thread.id,
        resolved: thread.is_resolved,
        url: first.url.clone(),
        first_comment_id: first.database_id,
        first_author,
        first_is_bot,
        first_body: first.body.clone(),
        bodies: thread
            .comments
            .nodes
            .iter()
            .map(|c| c.body.clone())
            .collect(),
    })
}

/// One page of the query, from `cursor`.
fn fetch(gh: &dyn Gh, link: &PullLink, base: &str, cursor: Option<&str>) -> Result<Data, Error> {
    let variables = json!({"owner": link.owner, "name": link.repo, "n": link.number, "base": base, "cursor": cursor});
    serde_json::from_value(graphql(gh, QUERY, &variables)?)
        .map_err(|error| Error::Json(error.to_string()))
}
