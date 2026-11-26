//! Fixtures of the tests of `edit`.

use std::path::PathBuf;

use crate::error::Error;
use crate::gh::FakeGh;

pub(super) const URL: &str = "https://github.com/o/r/issues/7";
pub(super) const MARKER: &str = "<!-- taskrock:plan:T1 -->";
pub(super) const LOOKUP: &str = r#"{"data":{"repository":{"issue":{"projectItems":{"nodes":[{"project":{"number":10,"owner":{"login":"o"}}}]}}}}}"#;
pub(super) const FIELDS: &str =
    r#"[[{"id":7,"name":"Status","options":[]},{"id":9,"name":"Parent issue"}]]"#;
pub(super) const NO_CONFIG: &str = "gh: Not Found (HTTP 404)";
pub(super) const AGENT: &str =
    r#"{"issue_field_name":"Agent","single_select_option":{"name":"Owner"}}"#;

pub(super) fn body(outcome: &str, skip: &str) -> String {
    let all = [
        ("Outcome", outcome),
        ("Decisions already made", "d"),
        ("Steps", "s"),
        ("Out of scope", "o"),
    ];
    let text = all
        .iter()
        .filter(|(name, _)| *name != skip)
        .fold(String::new(), |text, (name, section)| {
            text + &format!("## {name}\n\n{section}\n\n")
        });
    format!("{text}## Done when\n\n- [ ] `cargo test` passes\n")
}

pub(super) fn file(name: &str, text: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("taskrock-edit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create the directory");
    let path = dir.join(name);
    std::fs::write(&path, text).expect("write the body");
    path
}

pub(super) fn item(status: &str, agent: &str) -> String {
    let status = if status.is_empty() {
        "null".to_owned()
    } else {
        format!(r#"{{"name":{{"raw":"{status}"}}}}"#)
    };
    format!(
        r#"[[{{"id":7,"content_type":"Issue","content":{{"html_url":"{URL}","id":7,"title":"T","state":"open","type":{{"name":"Task"}},"sub_issues_summary":{{"total":0}},"issue_dependencies_summary":{{"blocked_by":0,"blocking":0}},"issue_field_values":[{agent}]}},"fields":[{{"name":"Status","value":{status}}},{{"name":"Parent issue","value":null}}]}}]]"#
    )
}

pub(super) fn issue(kind: &str, state: &str, old: &str) -> String {
    let kind = if kind.is_empty() {
        "null".to_owned()
    } else {
        format!(r#"{{"name":"{kind}"}}"#)
    };
    serde_json::json!({"type": serde_json::from_str::<serde_json::Value>(&kind).expect("json"), "state": state, "body": old}).to_string()
}

pub(super) fn fake(issue: &str, status: &str, agent: &str) -> FakeGh {
    FakeGh::new()
        .ok("issues/7", issue)
        .ok("graphql", LOOKUP)
        .ok("fields", FIELDS)
        .ok("items", &item(status, agent))
        .fail("contents/taskrock.toml", NO_CONFIG)
        .ok("PATCH", "{}")
        .ok("comments", "{}")
}

pub(super) fn has(gh: &FakeGh, what: &str) -> bool {
    gh.calls().iter().any(|(call, _)| call.contains(what))
}

pub(super) fn refused(error: Error) -> String {
    match error {
        Error::Refused(message) => message,
        other => panic!("not a refusal: {other}"),
    }
}
