//! The writes of a thread: a resolve that GitHub does not confirm, a failed reply, the report.

use serde_json::{Value, json};

use super::{HEAD, P3, bot, link, pull, reads, resolved, writes};
use crate::pr::EXIT_BLOCKED;
use crate::pr::threads::{Action, execute};

const URL: &str = "https://github.com/tailrocks/taskrock/pull/5#discussion_r1";

/// Defer the thread `A`, with `answer` as what GitHub says to the resolve.
fn answered(answer: &Value, json: bool) -> (String, u8) {
    let gh = reads(&[bot("A", 1, P3)])
        .ok("pulls/5/comments/1/replies", "{}")
        .ok("graphql", &json!({"data": answer}).to_string())
        .ok("pulls/5", &pull("open", HEAD));
    execute(&gh, &link(), &Action::Defer, json).expect("runs")
}

#[test]
fn threads_a_resolve_that_is_not_true_fails_and_is_not_counted() {
    for thread in [json!({"isResolved": false}), Value::Null] {
        let answer = json!({"resolveReviewThread": {"thread": thread}});
        let (out, code) = answered(&answer, true);
        assert_eq!(code, EXIT_BLOCKED, "{answer}");
        let report: Value = serde_json::from_str(&out).expect("JSON");
        assert_eq!(report["resolved"], json!([]), "{answer}");
        let message = format!("GitHub did not resolve the thread {URL}");
        assert_eq!(report["failed"], message, "{answer}");
    }
}

#[test]
fn threads_a_failed_reply_is_not_followed_by_a_resolve() {
    let gh = reads(&[bot("A", 1, P3)])
        .fail("pulls/5/comments/1/replies", "gh: Forbidden (HTTP 403)")
        .ok("graphql", &resolved())
        .ok("pulls/5", &pull("open", HEAD));
    let (out, code) = execute(&gh, &link(), &Action::Defer, false).expect("runs");
    assert_eq!(code, EXIT_BLOCKED);
    let made = writes(&gh);
    assert!(made.len() == 1 && made[0].starts_with("reply "), "{made:?}");
    let failed = out.lines().find(|line| line.starts_with("failed: "));
    let failed = failed.expect("a failed line");
    assert!(
        failed.starts_with(&format!("failed: {URL}: gh ")),
        "{failed}"
    );
    assert!(failed.contains("403"), "{failed}");
}

#[test]
fn threads_the_json_flag_reaches_the_report() {
    let done = json!({"resolveReviewThread": {"thread": {"isResolved": true}}});
    let (out, code) = answered(&done, true);
    let report: Value = serde_json::from_str(&out).expect("JSON");
    assert_eq!(
        report,
        json!({"schema": 1, "resolved": [URL], "failed": null})
    );
    assert_eq!(code, 0);
    assert_eq!(
        answered(&done, false).0,
        format!("resolved {URL}\nresolved 1 of 1 thread(s)")
    );
}
