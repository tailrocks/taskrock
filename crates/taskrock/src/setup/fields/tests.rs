//! Tests of the routing fields step.

use std::path::PathBuf;

use serde_json::{Value, json};

use super::{routing_fields, wanted};

mod update;
use crate::config::{self, Config, Model};
use crate::error::Error;
use crate::gh::FakeGh;
use crate::setup::Run;

pub(super) fn good() -> Config {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/config-good.toml");
    config::parse::read(&file).expect("good config")
}

pub(super) fn names(list: &[String]) -> Vec<&str> {
    list.iter().map(String::as_str).collect()
}

pub(super) fn field(id: u64, name: &str, visibility: &str, options: &[&str]) -> Value {
    let options: Vec<Value> = (1..)
        .zip(options)
        .map(|(n, name)| json!({"id": id * 100 + n, "name": name, "color": "red", "priority": n}))
        .collect();
    json!({"id": id, "name": name, "visibility": visibility, "options": options})
}

pub(super) fn members(id: u64, name: &str, options: &[&str]) -> Value {
    field(id, name, "organization_members_only", options)
}

pub(super) fn reply(fields: &[Value]) -> String {
    Value::Array(fields.to_vec()).to_string()
}

pub(super) fn run_step(
    gh: &FakeGh,
    config: &Config,
    dry_run: bool,
) -> (Result<(), Error>, Vec<String>) {
    let mut run = Run {
        gh,
        owner: "acme".to_owned(),
        dry_run,
        yes: false,
        lines: Vec::new(),
    };
    let result = routing_fields(&mut run, config);
    (result, run.lines)
}

/// All eight fields, complete, for the good config.
pub(super) fn complete(config: &Config) -> Vec<Value> {
    (1..)
        .zip(wanted(config))
        .map(|(id, (name, options))| members(id, name, &names(&options)))
        .collect()
}

#[test]
fn wanted_lists_the_options_of_the_eight_fields() {
    let list = wanted(&good());
    let fields: Vec<&str> = list.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        fields,
        [
            "Agent",
            "Review agent",
            "Runtime",
            "Review runtime",
            "Model",
            "Review model",
            "Effort",
            "Review effort"
        ]
    );
    assert_eq!(names(&list[0].1), ["Owner", "builder"]);
    assert_eq!(names(&list[1].1), ["builder", "reviewer"]);
    assert_eq!(names(&list[3].1), ["claude", "codex"]);
    assert_eq!(names(&list[5].1), ["small"]);
    assert_eq!(names(&list[7].1), ["low", "high"]);
}

#[test]
fn wanted_efforts_keep_first_appearance_without_repeats() {
    let mut config = good();
    config.routing.models.push(Model {
        name: "big".to_owned(),
        runtimes: vec!["claude".to_owned()],
        efforts: vec!["high".to_owned(), "max".to_owned()],
        usage: String::new(),
    });
    let list = wanted(&config);
    assert_eq!(names(&list[6].1), ["low", "high", "max"]);
    assert_eq!(names(&list[4].1), ["small", "big"]);
}

#[test]
fn missing_fields_are_created_with_graphql() {
    let mut gh = FakeGh::new()
        .ok("issue-fields", "[]")
        .ok("graphql", r#"{"data":{"organization":{"id":"O_1"}}}"#);
    for _ in 0..8 {
        gh = gh.ok("graphql", r#"{"data":{}}"#);
    }
    let (result, lines) = run_step(&gh, &good(), false);
    result.expect("step");
    assert_eq!(lines.len(), 8);
    assert_eq!(lines[0], "create field Agent");
    let calls = gh.calls();
    assert_eq!(calls.len(), 10);
    let input = calls[2].1.clone().expect("input");
    let sent: Value = serde_json::from_str(&input).expect("json");
    assert!(sent["query"].as_str().expect("query").contains(
        r#"options:[{name:"Owner",color:GRAY,priority:1},{name:"builder",color:GRAY,priority:2}]"#
    ));
    assert_eq!(sent["variables"]["o"], "O_1");
    assert_eq!(sent["variables"]["n"], "Agent");
}

#[test]
fn dry_run_writes_and_reads_nothing_more() {
    let gh = FakeGh::new().ok("issue-fields", "[]");
    let (result, lines) = run_step(&gh, &good(), true);
    result.expect("step");
    assert_eq!(
        lines
            .iter()
            .filter(|l| l.starts_with("create field "))
            .count(),
        8
    );
    assert_eq!(gh.calls().len(), 1);
}

#[test]
fn complete_fields_exist() {
    let config = good();
    let gh = FakeGh::new().ok("issue-fields", &reply(&complete(&config)));
    let (result, lines) = run_step(&gh, &config, false);
    result.expect("step");
    assert!(lines.iter().all(|line| line.starts_with("exists field ")));
    assert_eq!(gh.calls().len(), 1);
}
