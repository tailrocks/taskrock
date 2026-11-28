//! The paths that write nothing, or that GitHub refuses.

use serde_json::json;

use super::{Kind, OLD, Scene, run, writes};
use crate::error::Error;

fn nothing(gh: &crate::gh::FakeGh) {
    assert_eq!(writes(gh), Vec::<String>::new());
}

#[test]
fn merge_waits_for_bot_review() {
    let gh = Scene {
        codex_head: Some(OLD),
        ..Scene::ready()
    }
    .gh();
    let done = run(&gh).expect("not ready");
    assert_eq!(done.code, 10);
    assert_eq!(done.report.outcome, super::super::Outcome::NotReady);
    assert!(
        done.lines[0].contains("bot_review_pending"),
        "{:?}",
        done.lines
    );
    nothing(&gh);
}

#[test]
fn merge_of_a_blocked_pull_exits_20_and_writes_nothing() {
    let gh = Scene {
        passed: false,
        ..Scene::ready()
    }
    .gh();
    let done = run(&gh).expect("not ready");
    assert_eq!(done.code, 20);
    assert!(
        done.lines[0].contains("verdict_missing"),
        "{:?}",
        done.lines
    );
    nothing(&gh);
}

#[test]
fn merge_refuses_a_fork_before_it_reads_anything_else_or_writes() {
    let gh = Scene {
        kind: Kind::Fork,
        ..Scene::ready()
    }
    .gh();
    match run(&gh) {
        Err(Error::Refused(message)) => assert_eq!(message, "taskrock never merges a fork"),
        other => panic!("a fork is refused, got {:?}", other.map(|done| done.code)),
    }
    nothing(&gh);
}

#[test]
fn merge_exits_20_when_github_refuses_a_direct_merge() {
    for status in [405, 409, 422] {
        let stderr = format!("gh: Base branch was modified. (HTTP {status})");
        let gh = Scene::ready()
            .gh()
            .ok("pulls?base=", "[[]]")
            .fail("pulls/5/merge", &stderr);
        let done = run(&gh).expect("refused");
        let line = format!("GitHub refused the merge: Base branch was modified. (HTTP {status})");
        assert_eq!(done.lines, std::slice::from_ref(&line));
        assert_eq!(done.refusal, Some(line));
        assert_eq!(done.code, 20);
    }
}

#[test]
fn merge_exits_20_when_the_enqueue_has_a_graphql_error() {
    let errors = json!({"errors": [{"message": "Head sha does not match"}]}).to_string();
    let gh = Scene {
        queue: true,
        ..Scene::ready()
    }
    .gh()
    .ok("pulls?base=", "[[]]")
    .ok("graphql", &errors);
    let done = run(&gh).expect("refused");
    assert_eq!(
        done.lines,
        ["GitHub refused the merge: Head sha does not match"]
    );
    assert_eq!(done.code, 20);
    assert_eq!(writes(&gh).len(), 1);
}

#[test]
fn merge_does_not_call_a_server_error_a_refusal() {
    let gh = Scene::ready()
        .gh()
        .ok("pulls?base=", "[[]]")
        .fail("pulls/5/merge", "gh: Server Error (HTTP 500)");
    assert!(matches!(run(&gh), Err(Error::Gh { .. })));
}

#[test]
fn merge_waits_while_no_check_has_started_and_writes_nothing() {
    let gh = Scene {
        check_started: false,
        ..Scene::ready()
    }
    .gh();
    let done = run(&gh).expect("not ready");
    assert_eq!(done.code, 10);
    assert_eq!(done.report.outcome, super::super::Outcome::NotReady);
    assert!(done.lines[0].contains("checks_pending"), "{:?}", done.lines);
    nothing(&gh);
}
