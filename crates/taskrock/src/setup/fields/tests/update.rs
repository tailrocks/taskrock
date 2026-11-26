//! Tests of the update, rename, limit and visibility paths.

use serde_json::{Value, json};

use super::{complete, field, good, members, reply, run_step};
use crate::gh::FakeGh;

#[test]
fn missing_options_are_added_with_every_existing_id() {
    let config = good();
    let mut fields = complete(&config);
    fields[0] = members(1, "Agent", &["Owner", "Extra"]);
    let gh = FakeGh::new()
        .ok("issue-fields", &reply(&fields))
        .ok("-X PATCH orgs/acme/issue-fields/1", "{}");
    let (result, lines) = run_step(&gh, &config, false);
    result.expect("step");
    assert_eq!(lines[0], "update field Agent (add builder)");
    let calls = gh.calls();
    assert_eq!(calls.len(), 2);
    let body: Value = serde_json::from_str(&calls[1].1.clone().expect("body")).expect("json");
    assert_eq!(
        body,
        json!({"options": [
            {"id": 101, "name": "Owner", "color": "red", "priority": 1},
            {"id": 102, "name": "Extra", "color": "red", "priority": 2},
            {"name": "builder", "color": "gray", "priority": 3},
        ]})
    );
}

#[test]
fn a_field_of_sizes_called_effort_is_renamed() {
    let config = good();
    let mut fields = complete(&config);
    fields[6] = members(7, "Effort", &["S", "M", "L"]);
    let gh = FakeGh::new()
        .ok("issue-fields", &reply(&fields))
        .ok("-X PATCH orgs/acme/issue-fields/7", "{}")
        .ok("graphql", r#"{"data":{"organization":{"id":"O_1"}}}"#)
        .ok("graphql", r#"{"data":{}}"#);
    let (result, lines) = run_step(&gh, &config, false);
    result.expect("step");
    assert_eq!(lines[0], "update field Effort (rename to Size)");
    assert!(lines.contains(&"create field Effort".to_owned()));
    assert_eq!(gh.calls()[1].1.as_deref(), Some(r#"{"name":"Size"}"#));
}

#[test]
fn effort_and_size_together_stop_before_any_write() {
    let mut fields = vec![members(1, "Effort", &["S"]), members(2, "Size", &["S"])];
    fields.extend(complete(&good()).split_off(1));
    let gh = FakeGh::new().ok("issue-fields", &reply(&fields));
    let (result, _) = run_step(&gh, &good(), false);
    let error = result.expect_err("conflict");
    assert!(error.to_string().contains("Rename one of them"));
    assert_eq!(gh.calls().len(), 1);
}

#[test]
fn more_than_25_fields_stop_before_any_write() {
    let others: Vec<Value> = (1..=25)
        .map(|id| members(id, &format!("Other {id}"), &[]))
        .collect();
    let gh = FakeGh::new().ok("issue-fields", &reply(&others));
    let (result, lines) = run_step(&gh, &good(), false);
    let error = result.expect_err("limit");
    assert_eq!(
        error.to_string(),
        "the organization would have 33 issue fields. The limit is 25"
    );
    assert_eq!(lines.len(), 0);
    assert_eq!(gh.calls().len(), 1);
}

#[test]
fn visibility_is_fixed_when_it_differs() {
    let config = good();
    let mut fields = complete(&config);
    fields[2] = field(3, "Runtime", "all", &["claude", "codex"]);
    let gh = FakeGh::new()
        .ok("issue-fields", &reply(&fields))
        .ok("-X PATCH orgs/acme/issue-fields/3", "{}");
    let (result, lines) = run_step(&gh, &config, false);
    result.expect("step");
    assert!(lines.contains(&"update field Runtime (visibility)".to_owned()));
    assert_eq!(
        gh.calls()[1].1.as_deref(),
        Some(r#"{"visibility":"organization_members_only"}"#)
    );
}
