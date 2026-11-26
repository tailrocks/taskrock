//! Tests of `taskrock setup`.

use std::path::PathBuf;

use super::{Action, Change, Options, fields, run};
use crate::config;
use crate::error::Error;
use crate::gh::FakeGh;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

fn options(file: &str, dry_run: bool) -> Options {
    Options {
        owner: "acme".to_owned(),
        file: Some(fixture(file)),
        dry_run,
        yes: false,
    }
}

fn org_types(names: &[&str]) -> String {
    let types: Vec<_> = names
        .iter()
        .map(|name| serde_json::json!({"name": name}))
        .collect();
    serde_json::Value::Array(types).to_string()
}

/// The eight routing fields of `config-good.toml`, all present and complete.
fn org_fields() -> String {
    let config = config::parse::read(&fixture("config-good.toml")).expect("good config");
    let fields: Vec<_> = (1..)
        .zip(fields::wanted(&config))
        .map(|(id, (name, options))| {
            let options: Vec<_> = (1..)
                .zip(options)
                .map(|(n, name)| serde_json::json!({"id": id * 100 + n, "name": name, "color": "gray", "priority": n}))
                .collect();
            serde_json::json!({"id": id, "name": name, "visibility": "organization_members_only", "options": options})
        })
        .collect();
    serde_json::Value::Array(fields).to_string()
}

const ALL: [&str; 5] = ["Initiative", "Epic", "Task", "Bug", "Feature"];

#[test]
fn change_displays_action_kind_and_name() {
    let change = Change {
        action: Action::Create,
        kind: "type",
        name: "Bug".to_owned(),
    };
    assert_eq!(change.to_string(), "create type Bug");
    assert_eq!(Action::Update.to_string(), "update");
    assert_eq!(Action::Delete.to_string(), "delete");
    assert_eq!(Action::Exists.to_string(), "exists");
}

#[test]
fn a_bad_config_writes_nothing() {
    let gh = FakeGh::new();
    let error = run(&gh, &options("config-bad.toml", false)).expect_err("bad config");
    assert!(matches!(error, Error::Check(3)));
    assert_eq!(gh.calls().len(), 0);
}

#[test]
fn a_missing_type_is_created() {
    let gh = FakeGh::new()
        .ok("issue-types", &org_types(&ALL[..4]))
        .ok("-X POST orgs/acme/issue-types", "{}")
        .ok("issue-fields", &org_fields());
    let out = run(&gh, &options("config-good.toml", false)).expect("setup");
    assert_eq!(out, "DONE 1 changes");
    let calls = gh.calls();
    assert_eq!(calls.len(), 3);
    assert_eq!(
        calls[1].1.as_deref(),
        Some(r#"{"is_enabled":true,"name":"Feature"}"#)
    );
}

#[test]
fn dry_run_makes_no_post() {
    let gh = FakeGh::new()
        .ok("issue-types", &org_types(&ALL[..4]))
        .ok("issue-fields", &org_fields());
    let out = run(&gh, &options("config-good.toml", true)).expect("setup");
    assert_eq!(out, "DONE 1 changes");
    assert_eq!(gh.calls().len(), 2);
}

#[test]
fn a_second_run_only_finds_existing_types() {
    let gh = FakeGh::new()
        .ok("issue-types", &org_types(&ALL))
        .ok("issue-fields", &org_fields());
    let out = run(&gh, &options("config-good.toml", false)).expect("setup");
    assert_eq!(out, "DONE 0 changes");
    assert_eq!(gh.calls().len(), 2);
}
