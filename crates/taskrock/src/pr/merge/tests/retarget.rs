//! When `pr merge` points the pull requests based on the head branch at the default branch:
//! after the pull request is merged, and on a run on a merged pull request. Never before.

use serde_json::json;

use super::{HEAD, Kind, Scene, run, writes};
use crate::gh::FakeGh;
use crate::pr::merge::Outcome;

/// Two open pull requests based on `feat/issue-5`, the head branch of the pull request.
fn children(gh: FakeGh) -> FakeGh {
    gh.ok(
        "pulls?base=feat/issue-5&state=open",
        r#"[[{"number":7},{"number":8}]]"#,
    )
    .ok("pulls/7", "{}")
    .ok("pulls/8", "{}")
}

fn patches() -> [String; 2] {
    let base = json!({"base": "main"});
    [7, 8].map(|number| format!("PATCH repos/tailrocks/taskrock/pulls/{number} {base}"))
}

fn merge() -> String {
    let body = json!({"merge_method": "squash", "sha": HEAD});
    format!("PUT repos/tailrocks/taskrock/pulls/5/merge {body}")
}

#[test]
fn merge_retargets_the_children_after_the_pull_request_merged() {
    let gh = children(Scene::ready().gh()).ok("pulls/5/merge", r#"{"sha":"abc1234"}"#);
    let done = run(&gh).expect("merged");
    assert_eq!(
        done.lines,
        [
            "merged as abc1234",
            "retargeted #7 to main",
            "retargeted #8 to main"
        ]
    );
    assert_eq!(done.report.retargeted, [7, 8]);
    let [seven, eight] = patches();
    assert_eq!(writes(&gh), [merge(), seven, eight]);
}

#[test]
fn merge_retargets_nothing_when_github_refuses() {
    let gh = children(Scene::ready().gh()).fail("pulls/5/merge", "gh: Conflict (HTTP 409)");
    let done = run(&gh).expect("refused");
    assert_eq!(done.report.outcome, Outcome::Refused);
    assert_eq!(done.code, 20);
    assert_eq!(done.report.retargeted, Vec::<u64>::new());
    assert_eq!(writes(&gh), [merge()]);
}

#[test]
fn merge_retargets_nothing_when_the_pull_request_is_queued() {
    let queue = Scene {
        queue: true,
        ..Scene::ready()
    };
    let gh = children(queue.gh()).ok(
        "graphql",
        r#"{"data":{"enqueuePullRequest":{"mergeQueueEntry":{"position":3}}}}"#,
    );
    let done = run(&gh).expect("queued");
    assert_eq!(done.report.outcome, Outcome::Queued);
    assert_eq!(done.code, 10);
    assert_eq!(done.report.retargeted, Vec::<u64>::new());
    assert_eq!(
        done.lines,
        [format!(
            "queued at position 3 (head {}); the pull requests based on feat/issue-5 are \
             retargeted when it merges: run pr merge again then",
            &HEAD[..7]
        )]
    );
    assert_eq!(
        writes(&gh),
        [format!(r#"graphql {{"head":"{HEAD}","id":"PR_5"}}"#)]
    );
}

#[test]
fn merge_retargets_the_children_that_are_left_on_already_merged() {
    let scene = Scene {
        kind: Kind::Merged,
        ..Scene::ready()
    };
    let gh = children(scene.gh());
    let done = run(&gh).expect("already merged");
    assert_eq!(done.report.outcome, Outcome::AlreadyMerged);
    assert_eq!(done.code, 0);
    assert_eq!(
        done.lines,
        [
            "already merged as c0ffee0123456789c0ffee0123456789c0ffee01",
            "retargeted #7 to main",
            "retargeted #8 to main"
        ]
    );
    assert_eq!(done.report.retargeted, [7, 8]);
    assert_eq!(writes(&gh), patches());
}

#[test]
fn merge_finds_no_child_when_github_retargeted_them_itself() {
    let scene = Scene {
        kind: Kind::Merged,
        ..Scene::ready()
    };
    let gh = scene.gh().ok("pulls?base=feat/issue-5&state=open", "[[]]");
    let done = run(&gh).expect("already merged");
    assert_eq!(done.report.retargeted, Vec::<u64>::new());
    assert_eq!(writes(&gh), Vec::<String>::new());
}

#[test]
fn merge_of_a_merged_fork_pull_retargets_nothing() {
    let scene = Scene {
        kind: Kind::MergedFork,
        ..Scene::ready()
    };
    let gh = children(scene.gh());
    let done = run(&gh).expect("already merged");
    assert_eq!(done.report.outcome, Outcome::AlreadyMerged);
    assert_eq!(done.report.retargeted, Vec::<u64>::new());
    assert_eq!(writes(&gh), Vec::<String>::new());
}
