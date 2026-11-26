//! Tests of the Status of the new issues.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::apply::status::set_statuses;
use crate::apply::{Created, create_issues};
use crate::draft::{self, Draft};
use crate::error::Error;
use crate::gh::FakeGh;
use crate::github::items::status_field;

use super::super::tests::{base, dry_options, has_call, options};

const FIELDS: &str = include_str!("../../../fixtures/project_fields.json");

fn folder() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/draft-triage")
}

/// The project fields, with the option `Triage` when `triage` is true.
fn fields(triage: bool) -> String {
    let mut fields: Vec<Value> = serde_json::from_str(FIELDS).expect("fixture");
    for field in &mut fields {
        if field["name"] == "Status"
            && triage
            && let Some(list) = field["options"].as_array_mut()
        {
            list.push(json!({"id":"abc123","name":{"raw":"Triage","html":"Triage"}}));
        }
    }
    serde_json::to_string(&fields).expect("fields")
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

/// The draft, with `T01` as issue 12 and `B1` as issue 13.
fn draft_and_found() -> (Draft, BTreeMap<String, Created>) {
    let draft = draft::parse::read(&folder()).expect("fixture is a draft");
    let mut found = BTreeMap::new();
    for (key, number) in [("T01", 12), ("B1", 13)] {
        let link = format!("https://github.com/tailrocks/holla/issues/{number}");
        let link = link.parse().expect("link");
        found.insert(
            key.to_owned(),
            Created {
                link,
                id: number + 100,
            },
        );
    }
    (draft, found)
}

/// A fake that answers the field read, then two reads of the items.
fn fake(first: &[Value], second: &[Value]) -> FakeGh {
    let fields = format!("[{}]", fields(true));
    FakeGh::new()
        .ok("7/fields", &fields)
        .ok("7/fields", &fields)
        .ok("7/items?", &json!([first]).to_string())
        .ok("7/fields", &fields)
        .ok("7/items?", &json!([second]).to_string())
        .ok("7/items/", "{}")
        .ok("7/items/", "{}")
        .ok("7/items/", "{}")
}

fn patches(gh: &FakeGh) -> Vec<String> {
    gh.calls()
        .into_iter()
        .filter(|(call, _)| call.contains("PATCH"))
        .map(|(call, input)| format!("{call} {}", input.unwrap_or_default()))
        .collect()
}

#[test]
fn bug_goes_to_triage_and_task_to_todo() {
    let (draft, found) = draft_and_found();
    let after = [item(12, Some("Todo")), item(13, Some("Todo"))];
    let gh = fake(&[item(12, None), item(13, Some("Todo"))], &after);
    let lines = set_statuses(&gh, &draft, &found, &options()).expect("statuses");
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
    let (draft, found) = draft_and_found();
    let triage = [item(12, Some("Todo")), item(13, Some("Triage"))];
    let gh = fake(&triage, &triage);
    let lines = set_statuses(&gh, &draft, &found, &options()).expect("statuses");
    assert!(lines.is_empty(), "nothing to change: {lines:?}");
    assert_eq!(patches(&gh).len(), 0, "no PATCH is sent");
}

#[test]
fn dry_run_makes_no_call() {
    let (draft, found) = draft_and_found();
    let gh = FakeGh::new();
    let lines = set_statuses(&gh, &draft, &found, &dry_options()).expect("dry run");
    assert_eq!(lines, ["would set B1 to Triage"]);
    assert!(gh.calls().is_empty(), "a dry run makes no call");
}

#[test]
fn project_without_triage_stops_before_any_creation() {
    let gh = base().ok("7/fields", &format!("[{}]", fields(false)));
    let error = create_issues(&gh, &folder(), &options()).expect_err("no Triage");
    assert!(
        matches!(&error, Error::Project(message) if message
            == "the project has no Status option Triage. Run taskrock setup"),
        "{error:?}"
    );
    assert!(!has_call(&gh, "POST"), "nothing is created");
}

#[test]
fn status_field_reads_triage_only_when_the_option_exists() {
    let project = draft_and_found().0.plan.project;
    let with = FakeGh::new().ok("7/fields", &format!("[{}]", fields(true)));
    let without = FakeGh::new().ok("7/fields", &format!("[{}]", fields(false)));
    let triage = status_field(&with, &project).expect("fields").triage;
    assert_eq!(triage.as_deref(), Some("abc123"));
    assert_eq!(status_field(&without, &project).expect("f").triage, None);
}
