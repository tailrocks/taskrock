//! The paths that merge or queue the pull request.

use serde_json::json;

use super::{HEAD, Kind, Scene, run, writes};
use crate::pr::merge::write::encode;

#[test]
fn merge_queues_the_head_when_the_base_has_a_queue() {
    let gh = Scene {
        queue: true,
        ..Scene::ready()
    }
    .gh()
    .ok(
        "graphql",
        r#"{"data":{"enqueuePullRequest":{"mergeQueueEntry":{"position":3}}}}"#,
    );
    let done = run(&gh).expect("queued");
    assert!(
        done.lines[0].starts_with(&format!("queued at position 3 (head {})", &HEAD[..7])),
        "{:?}",
        done.lines
    );
    assert_eq!(done.code, 10);
    assert_eq!(
        writes(&gh),
        [format!(r#"graphql {{"head":"{HEAD}","id":"PR_5"}}"#)]
    );
}

#[test]
fn merge_squashes_at_the_checked_head_without_a_queue() {
    let gh = Scene::ready()
        .gh()
        .ok("pulls?base=", "[[]]")
        .ok("pulls/5/merge", r#"{"sha":"abc1234","merged":true}"#);
    let done = run(&gh).expect("merged");
    assert_eq!(done.lines, ["merged as abc1234"]);
    assert_eq!(done.code, 0);
    let body = json!({"merge_method": "squash", "sha": HEAD});
    assert_eq!(
        writes(&gh),
        [format!("PUT repos/tailrocks/taskrock/pulls/5/merge {body}")]
    );
}

#[test]
fn merge_of_a_merged_pull_without_children_says_so_and_writes_nothing() {
    let gh = Scene {
        kind: Kind::Merged,
        ..Scene::ready()
    }
    .gh()
    .ok("pulls?base=", "[[]]");
    let done = run(&gh).expect("already merged");
    assert_eq!(
        done.lines,
        ["already merged as c0ffee0123456789c0ffee0123456789c0ffee01"]
    );
    assert_eq!(done.code, 0);
    assert_eq!(writes(&gh), Vec::<String>::new());
}

#[test]
fn merge_json_report_has_the_documented_keys() {
    let gh = Scene::ready()
        .gh()
        .ok("pulls?base=", r#"[[{"number":7}]]"#)
        .ok("pulls/7", "{}")
        .ok("pulls/5/merge", r#"{"sha":"abc1234"}"#);
    let report = serde_json::to_value(run(&gh).expect("merged").report).expect("JSON");
    assert_eq!(
        report,
        json!({"schema": 1, "repo": "tailrocks/taskrock", "number": 5, "outcome": "merged",
            "head": HEAD, "merge_commit": "abc1234", "retargeted": [7],
            "retarget_failures": [], "retarget_error": null})
    );
}

#[test]
fn merge_encodes_the_branch_name_in_the_query() {
    assert_eq!(encode("feat/a#b&c+d e"), "feat/a%23b%26c%2Bd%20e");
}
