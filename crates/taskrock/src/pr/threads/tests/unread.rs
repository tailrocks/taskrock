//! A thread read that is not complete: no thread is selected and nothing is written.

use serde_json::json;

use super::{HEAD, P3, base, bot, graph, link, pull, reads, resolve, resolved, thread, writes};
use crate::error::Error;
use crate::gh::FakeGh;
use crate::pr::EXIT_DONE;
use crate::pr::threads::{Action, run};

/// Both actions refuse with `problem` in the message, and no write is made.
fn refused(scripted: impl Fn() -> FakeGh, problem: &str) {
    for action in [Action::Defer, resolve("A", "why")] {
        let gh = scripted();
        let error = run(&gh, &link(), &action, false).expect_err("refused");
        assert_eq!(error.exit_code(), 3);
        let Error::Refused(message) = &error else {
            panic!("not a refusal: {error}");
        };
        assert!(message.starts_with("the review threads were not read in full"));
        assert!(message.contains(problem), "{message}");
        assert_eq!(writes(&gh), Vec::<String>::new());
        assert!(gh.calls().iter().all(|(args, _)| !args.contains("-X")));
    }
}

#[test]
fn threads_a_failed_graphql_read_refuses() {
    refused(
        || base().fail("graphql", "gh: Bad Gateway (HTTP 502)"),
        "HTTP 502",
    );
}

#[test]
fn threads_a_failed_later_page_refuses() {
    let first = graph(&[bot("A", 1, P3)], Some("C"));
    refused(
        || {
            base()
                .ok("graphql", &first)
                .fail("graphql", "gh: Bad Gateway (HTTP 502)")
        },
        "later pages",
    );
}

#[test]
fn threads_a_thread_with_over_100_comments_refuses() {
    let mut long = bot("A", 1, P3);
    long["comments"]["totalCount"] = json!(101);
    refused(|| reads(&[long.clone()]), "101 comments");
}

#[test]
fn threads_another_read_problem_does_not_block() {
    let empty = thread("E", 7, ("review-bot[bot]", "Bot"), &[], false);
    let gh = reads(&[empty, bot("A", 1, P3)])
        .ok("pulls/5/comments/1/replies", "{}")
        .ok("graphql", &resolved())
        .ok("pulls/5", &pull("open", HEAD));
    assert_eq!(
        run(&gh, &link(), &Action::Defer, false).expect("runs"),
        EXIT_DONE
    );
    assert_eq!(writes(&gh).len(), 2);
}
