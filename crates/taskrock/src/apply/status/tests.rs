//! Tests of the Status of the new issues.

use std::path::PathBuf;

use serde_json::{Value, json};

use crate::apply::create_issues;
use crate::apply::status::set_statuses;
use crate::error::Error;
use crate::gh::FakeGh;

mod rerun;

use super::super::tests::{base, draft_and_found, dry_options, has_call, options};

const FIELDS: &str = include_str!("../../../fixtures/project_fields.json");

/// The project fields as one page, with the option `Triage` when asked.
fn fields(triage: bool) -> String {
    let mut fields: Vec<Value> = serde_json::from_str(FIELDS).expect("fixture");
    let status = fields.iter_mut().find(|field| field["name"] == "Status");
    let list = status.and_then(|field| field["options"].as_array_mut());
    if let (true, Some(list)) = (triage, list) {
        list.push(json!({"id":"abc123","name":{"raw":"Triage"}}));
    }
    json!([fields]).to_string()
}

/// One project item of issue `number` with this Status.
fn item(number: u64, status: Option<&str>) -> Value {
    let value = status.map(|name| json!({"name":{"raw":name}}));
    json!({"id":number * 10,"content_type":"Issue","content":{
        "html_url":format!("https://github.com/tailrocks/holla/issues/{number}"),
        "id":number + 100,"title":"t","state":"open","type":null,
        "sub_issues_summary":{"total":0},
        "issue_dependencies_summary":{"blocked_by":0,"blocking":0}},
        "fields":[{"name":"Status","value":value}]})
}

/// A fake that answers the field read, then two reads of the items.
fn fake(first: &[Value], second: &[Value]) -> FakeGh {
    let fields = fields(true);
    let gh = FakeGh::new()
        .ok("7/fields", &fields)
        .ok("7/fields", &fields)
        .ok("7/items?", &json!([first]).to_string())
        .ok("7/fields", &fields)
        .ok("7/items?", &json!([second]).to_string());
    (0..3).fold(gh, |gh, _| gh.ok("7/items/", "{}"))
}

fn patches(gh: &FakeGh) -> Vec<String> {
    let calls = gh.calls().into_iter();
    let sent = calls.filter(|(call, _)| call.contains("PATCH"));
    sent.map(|(call, body)| format!("{call} {}", body.unwrap_or_default()))
        .collect()
}

#[test]
fn bug_goes_to_triage_and_task_to_todo() {
    let after = [item(12, Some("Todo")), item(13, Some("Todo"))];
    let (lines, gh) = run_with(&[], &[item(12, None), item(13, Some("Todo"))], &after);
    assert_eq!(
        lines,
        [
            "status: #13 Triage",
            "status: #12 Todo",
            "status: #13 Triage"
        ]
    );
    let sent = patches(&gh);
    assert!(sent[0].contains("items/130") && sent[0].contains("abc123"));
    assert!(sent[1].contains("items/120") && sent[1].contains("f75ad846"));
    assert!(sent[2].contains("items/130") && sent[2].contains("abc123"));
}

#[test]
fn second_read_sets_a_bug_again_only_when_it_left_triage() {
    let triage = [item(12, Some("Todo")), item(13, Some("Triage"))];
    let (lines, gh) = run_with(&[], &triage, &triage);
    assert!(lines.is_empty() && patches(&gh).is_empty(), "{lines:?}");
}

/// Runs `set_statuses` on T01 (#12) and B1 (#13). The `existing` keys were
/// found, not created.
fn run_with(existing: &[&str], first: &[Value], second: &[Value]) -> (Vec<String>, FakeGh) {
    let (draft, mut found) = draft_and_found("draft-triage", &[("T01", 12), ("B1", 13)]);
    for key in existing {
        found
            .entry((*key).to_owned())
            .and_modify(|c| c.fresh = false);
    }
    let gh = fake(first, second);
    let lines = set_statuses(&gh, &draft, &found, &options()).expect("statuses");
    (lines, gh)
}

#[test]
fn dry_run_makes_no_call() {
    let (draft, found) = draft_and_found("draft-triage", &[("T01", 12), ("B1", 13)]);
    let gh = FakeGh::new();
    let lines = set_statuses(&gh, &draft, &found, &dry_options()).expect("dry run");
    assert_eq!(lines, ["would set B1 to Triage"]);
    assert!(gh.calls().is_empty(), "a dry run makes no call");
}

#[test]
fn dry_run_skips_a_found_bug() {
    let (draft, mut found) = draft_and_found("draft-triage", &[("T01", 12), ("B1", 13)]);
    found.entry("B1".to_owned()).and_modify(|c| c.fresh = false);
    let gh = FakeGh::new();
    let lines = set_statuses(&gh, &draft, &found, &dry_options()).expect("dry run");
    assert_eq!(lines, Vec::<String>::new());
    assert!(gh.calls().is_empty(), "a dry run makes no call");
}

fn draft_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/draft-triage")
}

#[test]
fn project_without_triage_stops_before_any_creation() {
    let gh = base().ok("7/fields", &fields(false));
    let error = create_issues(&gh, &draft_dir(), &options()).expect_err("no Triage");
    assert!(
        matches!(&error, Error::Project(message) if message
            == "the project has no Status option Triage. Run taskrock setup"),
        "{error:?}"
    );
    assert!(!has_call(&gh, "POST"), "nothing is created");
}

#[test]
fn task_only_draft_needs_no_triage_and_reads_the_items_once() {
    let (mut draft, found) = draft_and_found("draft-triage", &[("T01", 12), ("B1", 13)]);
    draft.issues.retain(|issue| issue.key == "T01");
    let no_triage = fields(false);
    let items = json!([[item(12, None)]]).to_string();
    let gh = FakeGh::new().ok("7/fields", &no_triage);
    let gh = gh.ok("7/fields", &no_triage).ok("7/items?", &items);
    let gh = gh.ok("7/items/", "{}");
    let lines = set_statuses(&gh, &draft, &found, &options()).expect("statuses");
    assert_eq!(lines, ["status: #12 Todo"]);
}
