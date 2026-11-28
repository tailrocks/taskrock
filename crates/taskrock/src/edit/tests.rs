//! Tests of `edit`.

use crate::error::Error;
use crate::gh::FakeGh;

use super::run;
use super::support::{AGENT, MARKER, URL, body, fake, file, has, issue, refused};

#[test]
fn todo_task_is_edited() {
    let old = format!("{}\n\n{MARKER}", body("a", ""));
    let gh = fake(&issue("Task", "open", &old), "Todo", "");
    let (_dir, path) = file("todo.md", &body("b", ""));
    assert_eq!(
        run(&gh, URL, &path, false).expect("edit"),
        format!("edited {URL}")
    );
    let calls = gh.calls();
    let sent = calls
        .iter()
        .find(|(call, _)| call.contains("PATCH"))
        .expect("patch");
    assert!(sent.0.contains("repos/o/r/issues/7"));
    assert!(!has(&gh, "comments"));
}

#[test]
fn missing_section_is_refused_before_the_write() {
    let gh = fake(&issue("Task", "open", &body("a", "")), "Todo", "");
    let (_dir, path) = file("missing.md", &body("a", "Steps"));
    let message = refused(run(&gh, URL, &path, false).expect_err("refused"));
    assert_eq!(
        message,
        "the body breaks the Task format: the section ## Steps is missing"
    );
    assert!(!has(&gh, "PATCH"));
}

#[test]
fn triage_checks_only_the_file_sections() {
    let old = issue("Bug", "open", "x");
    let gh = fake(&old, "Triage", "");
    let text = "## What happens\n\na\n\n## Expected\n\nb\n\n## How to reproduce\n\nc\n\n## Environment\n\nd\n";
    let (_dir, path) = file("triage.md", text);
    let gh_ok = run(&gh, URL, &path, false);
    assert!(gh_ok.is_ok(), "{gh_ok:?}");
}

#[test]
fn frozen_change_is_refused() {
    let gh = fake(&issue("Task", "open", &body("a", "")), "In Progress", "");
    let (_dir, path) = file("frozen.md", &body("b", ""));
    let message = refused(run(&gh, URL, &path, false).expect_err("refused"));
    assert_eq!(
        message,
        "the section ## Outcome is frozen after In Progress. Use --replan"
    );
    assert!(!has(&gh, "PATCH"));
}

#[test]
fn unfrozen_change_in_progress_passes_without_replan() {
    let gh = fake(&issue("Task", "open", &body("a", "")), "In Progress", "");
    let text = body("a", "").replace("## Steps\n\ns", "## Steps\n\nnew");
    let (_dir, path) = file("steps.md", &text);
    run(&gh, URL, &path, false).expect("edit");
    assert!(!has(&gh, "comments"));
}

#[test]
fn replan_posts_the_comment() {
    let gh = fake(&issue("Task", "open", &body("a", "")), "In Progress", "");
    let (_dir, path) = file("replan.md", &body("b", ""));
    run(&gh, URL, &path, true).expect("edit");
    let calls = gh.calls();
    let comment = calls
        .iter()
        .find(|(call, _)| call.contains("comments"))
        .expect("comment");
    let sent: serde_json::Value =
        serde_json::from_str(comment.1.as_deref().expect("input")).expect("json");
    assert_eq!(
        sent["body"],
        "_Written by an agent._\n\nReplanned: the sections Outcome changed."
    );
    assert!(has(&gh, "PATCH"));
}

#[test]
fn closed_issue_is_frozen() {
    let gh = fake(&issue("Task", "closed", &body("a", "")), "Todo", "");
    let (_dir, path) = file("closed.md", &body("b", ""));
    assert!(refused(run(&gh, URL, &path, false).expect_err("refused")).contains("is frozen"));
}

#[test]
fn marker_is_kept() {
    let old = format!("{}\n{MARKER}\n", body("a", ""));
    let gh = fake(&issue("Task", "open", &old), "Todo", "");
    let (_dir, path) = file("marker.md", &body("a", ""));
    run(&gh, URL, &path, false).expect("edit");
    let calls = gh.calls();
    let sent = calls
        .iter()
        .find(|(call, _)| call.contains("PATCH"))
        .expect("patch");
    let sent: serde_json::Value =
        serde_json::from_str(sent.1.as_deref().expect("input")).expect("json");
    let text = sent["body"].as_str().expect("body");
    assert!(text.ends_with(&format!("passes\n\n{MARKER}")), "{text}");
}

#[test]
fn front_matter_is_refused_before_any_call() {
    let gh = FakeGh::new();
    let (_dir, path) = file("front.md", "+++\nx\n+++\n");
    let error = run(&gh, URL, &path, false).expect_err("refused");
    assert!(
        matches!(&error, Error::Draft { message, .. } if message == "edit takes a body without front matter")
    );
    assert_eq!(gh.calls().len(), 0);
}

#[test]
fn unknown_or_missing_type_is_refused() {
    for (kind, name) in [("Saga", "Saga"), ("", "none")] {
        let gh = fake(&issue(kind, "open", "x"), "Todo", "");
        let (_dir, path) = file("kind.md", &body("a", ""));
        let message = refused(run(&gh, URL, &path, false).expect_err("refused"));
        assert_eq!(message, format!("the config has no issue type {name}"));
    }
}

#[test]
fn owner_task_uses_the_owner_format_whatever_the_type() {
    for kind in ["Task", "Saga", ""] {
        let gh = fake(&issue(kind, "open", "x"), "Todo", AGENT);
        let (_dir, path) = file("owner.md", &body("a", ""));
        let message = refused(run(&gh, URL, &path, false).expect_err("refused"));
        assert!(
            message.contains("## Question is missing"),
            "{kind}: {message}"
        );
    }
}
