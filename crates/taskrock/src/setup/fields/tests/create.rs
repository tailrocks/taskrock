//! Tests of the visibility a created field gets.

use serde_json::Value;

use super::{complete, field, good, reply, run_step};
use crate::config::Visibility;
use crate::gh::FakeGh;

/// The GraphQL query of the first `createIssueField` call.
fn create_query(gh: &FakeGh) -> String {
    let input = gh.calls()[2].1.clone().expect("input");
    let sent: Value = serde_json::from_str(&input).expect("json");
    sent["query"].as_str().expect("query").to_owned()
}

fn empty_org(visibility: Visibility) -> (FakeGh, crate::config::Config) {
    let mut config = good();
    config.routing.visibility = visibility;
    let mut gh = FakeGh::new()
        .ok("issue-fields", "[]")
        .ok("graphql", r#"{"data":{"organization":{"id":"O_1"}}}"#);
    for _ in 0..8 {
        gh = gh.ok("graphql", r#"{"data":{}}"#);
    }
    (gh, config)
}

#[test]
fn restricted_fields_are_created_restricted() {
    let (gh, config) = empty_org(Visibility::Restricted);
    run_step(&gh, &config, false).0.expect("step");
    assert!(create_query(&gh).contains("visibility:RESTRICTED,options:"));
}

#[test]
fn a_second_run_with_all_prints_only_exists() {
    let (gh, config) = empty_org(Visibility::All);
    let (result, lines) = run_step(&gh, &config, false);
    result.expect("first run");
    assert_eq!(lines.len(), 8);
    assert!(create_query(&gh).contains("visibility:ALL,options:"));
    let created: Vec<Value> = complete(&config)
        .iter()
        .map(|f| {
            let options: Vec<&str> = f["options"]
                .as_array()
                .expect("options")
                .iter()
                .map(|o| o["name"].as_str().expect("name"))
                .collect();
            field(
                f["id"].as_u64().expect("id"),
                f["name"].as_str().expect("name"),
                "all",
                &options,
            )
        })
        .collect();
    let second = FakeGh::new().ok("issue-fields", &reply(&created));
    let (result, lines) = run_step(&second, &config, false);
    result.expect("second run");
    assert_eq!(lines.len(), 8);
    assert!(lines.iter().all(|line| line.starts_with("exists field ")));
    assert_eq!(second.calls().len(), 1);
}
