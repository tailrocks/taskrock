//! Tests of `post`.

use std::path::PathBuf;

use crate::error::Error;
use crate::gh::FakeGh;

use super::post;

const URL: &str = "https://github.com/o/r/issues/7";
const GOOD: &str = "## Done\n\na\n\n## Verified\n\nb\n\n## State\n\nc\n\n## Next step\n\nd\n";
const ANSWER: &str = r#"{"html_url":"https://github.com/o/r/issues/7#issuecomment-1"}"#;

fn note(name: &str, text: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("taskrock-handoff-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create the directory");
    let path = dir.join(name);
    std::fs::write(&path, text).expect("write the note");
    path
}

#[test]
fn good_note_posts_once_with_the_agent_line_first() {
    let path = note("good.md", GOOD);
    let gh = FakeGh::new().ok("POST", ANSWER);
    let out = post(&gh, URL, &path).expect("post succeeds");
    assert_eq!(
        out,
        "note posted https://github.com/o/r/issues/7#issuecomment-1"
    );
    let calls = gh.calls();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].0.contains("repos/o/r/issues/7/comments"));
    let sent: serde_json::Value =
        serde_json::from_str(calls[0].1.as_deref().expect("input")).expect("json");
    let expected = format!("_Written by an agent._\n\n{GOOD}");
    assert_eq!(sent["body"], expected.as_str());
}

#[test]
fn note_without_state_makes_no_call() {
    let text = GOOD.replace("## State\n\nc\n\n", "");
    let path = note("no-state.md", &text);
    let gh = FakeGh::new();
    let error = post(&gh, URL, &path).expect_err("the note breaks a rule");
    match error {
        Error::Draft { file, message } => {
            assert_eq!(file, path.display().to_string());
            assert_eq!(message, "the section ## State is missing");
        }
        other => panic!("wrong error: {other:?}"),
    }
    assert_eq!(gh.calls().len(), 0);
}

#[test]
fn several_broken_rules_are_joined() {
    let path = note("short.md", "## Done\n\nx\n");
    let gh = FakeGh::new();
    let error = post(&gh, URL, &path).expect_err("the note breaks rules");
    let text = error.to_string();
    assert!(text.contains("## Verified is missing") || text.contains("## Verified"));
    assert!(text.contains("; "));
    assert_eq!(gh.calls().len(), 0);
}

#[test]
fn bad_link_returns_link_error() {
    let path = note("link.md", GOOD);
    let gh = FakeGh::new();
    let error = post(&gh, "not a link", &path).expect_err("bad link");
    assert!(matches!(error, Error::Link(_)));
    assert_eq!(gh.calls().len(), 0);
}

#[test]
fn missing_file_returns_draft_error() {
    let gh = FakeGh::new();
    let path = PathBuf::from("/nonexistent/taskrock-note.md");
    let error = post(&gh, URL, &path).expect_err("no file");
    assert!(matches!(error, Error::Draft { .. }));
    assert_eq!(gh.calls().len(), 0);
}
