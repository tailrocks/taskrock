//! Tests of the JSON of a verdict.

use serde_json::Value;

use super::json;
use super::tests::recorded;

fn sorted_keys(value: &Value) -> Vec<&str> {
    let mut keys: Vec<&str> = value
        .as_object()
        .expect("an object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}

#[test]
fn status_render_json_has_the_documented_keys() {
    let (link, snapshot, verdict) = recorded(3033);
    let raw = json(&link, &snapshot, &verdict);
    assert!(raw.contains("\n  \"schema\": 1,"), "pretty-printed");
    let value: Value = serde_json::from_str(&raw).expect("JSON");
    assert_eq!(
        sorted_keys(&value),
        [
            "findings", "head", "next", "number", "repo", "schema", "state", "threads"
        ]
    );
    assert_eq!(value["repo"], "tailrocks/velnor-new");
    assert_eq!(value["number"], 3033);
    assert_eq!(value["state"], "BLOCKED");
    assert_eq!(value["head"], verdict.head.as_str());
    let finding = &value["findings"][1];
    assert_eq!(
        sorted_keys(finding),
        ["code", "detail", "items", "kind", "next", "owner"]
    );
    assert_eq!(finding["code"], "threads_blocking");
    assert_eq!(finding["kind"], "blocker");
    assert_eq!(finding["items"].as_array().expect("items").len(), 12);
}

#[test]
fn status_render_json_lists_the_unresolved_threads() {
    let (link, snapshot, verdict) = recorded(3033);
    let value: Value = serde_json::from_str(&json(&link, &snapshot, &verdict)).expect("JSON");
    let open = snapshot.threads.iter().filter(|t| !t.resolved).count();
    let threads = value["threads"].as_array().expect("threads");
    assert_eq!(threads.len(), open);
    assert_eq!(
        sorted_keys(&threads[0]),
        ["author", "class", "id", "severity", "url"]
    );
    assert_eq!(threads[0]["class"], "blocking");
}

#[test]
fn status_render_json_severity_is_null_when_unreadable() {
    let (link, mut snapshot, verdict) = recorded(3033);
    let open = snapshot
        .threads
        .iter()
        .position(|t| !t.resolved)
        .expect("an open thread");
    snapshot.threads[open].first_body = "no badge here".into();
    let value: Value = serde_json::from_str(&json(&link, &snapshot, &verdict)).expect("JSON");
    let first = value["threads"].as_array().expect("threads")[0].clone();
    assert_eq!(first["severity"], Value::Null);
}
