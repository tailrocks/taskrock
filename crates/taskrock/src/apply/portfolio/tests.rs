//! Tests of adding the initiatives to the portfolio project.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::apply::portfolio::add_to_portfolio;
use crate::apply::{Created, create_issues};
use crate::draft::{self, Draft};
use crate::error::Error;
use crate::gh::FakeGh;

use super::super::tests::{base, dry_options, has_call, options};

const FIELDS: &str = include_str!("../../../fixtures/project_fields.json");

fn folder() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/draft-portfolio")
}

/// The draft, with `I1` as issue 11 and `T01` as issue 12.
fn draft_and_found() -> (Draft, BTreeMap<String, Created>) {
    let draft = draft::parse::read(&folder()).expect("fixture is a draft");
    let mut found = BTreeMap::new();
    for (key, number) in [("I1", 11), ("T01", 12)] {
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

/// The projects of the organization: a delivery project with the key
/// `roadmap`, and `9` with the marker of `kind` and `key`.
fn projects(key: &str, kind: &str) -> String {
    let readme = format!("Roadmap.\n\n<!-- taskrock:project key={key} kind={kind} -->");
    let delivery = "<!-- taskrock:project key=roadmap kind=delivery -->";
    json!({"data":{"organization":{"projectsV2":{"nodes":[
        {"id":"P1","number":7,"title":"holla","readme":delivery,"public":false,"closed":false},
        {"id":"P2","number":9,"title":"Roadmap","readme":readme,"public":false,"closed":false}
    ]}}}})
    .to_string()
}

/// One item of the portfolio project for issue `number`.
fn item(number: u64) -> Value {
    json!({"id":number * 10,"content_type":"Issue","content":{
        "html_url":format!("https://github.com/tailrocks/holla/issues/{number}"),
        "id":number + 100,"title":"t","state":"open","type":null,
        "sub_issues_summary":{"total":0},
        "issue_dependencies_summary":{"blocked_by":0,"blocking":0}},
        "fields":[]})
}

/// A fake that answers the project list and one read of the items.
fn fake(items: &[Value]) -> FakeGh {
    FakeGh::new()
        .ok("graphql", &projects("roadmap", "portfolio"))
        .ok("9/fields", &format!("[{FIELDS}]"))
        .ok("9/items?", &json!([items]).to_string())
        .ok("POST orgs/tailrocks/projectsV2/9/items", "{}")
}

fn posts(gh: &FakeGh) -> Vec<Value> {
    gh.calls()
        .into_iter()
        .filter(|(call, _)| call.contains("POST orgs/tailrocks/projectsV2/9/items"))
        .map(|(_, input)| serde_json::from_str(&input.expect("body")).expect("JSON"))
        .collect()
}

#[test]
fn portfolio_with_no_key_makes_no_call() {
    let (mut draft, found) = draft_and_found();
    draft.plan.portfolio = None;
    let gh = FakeGh::new();
    let lines = add_to_portfolio(&gh, &draft, &found, &options()).expect("no key");
    assert!(lines.is_empty(), "{lines:?}");
    assert!(gh.calls().is_empty(), "no call is made");
}

#[test]
fn portfolio_missing_stops_before_any_creation() {
    let gh = base().ok("graphql", &projects("other", "portfolio"));
    let error = create_issues(&gh, &folder(), &options()).expect_err("no portfolio");
    assert!(
        matches!(&error, Error::Project(message)
            if message == "no portfolio project with key roadmap. Run taskrock setup"),
        "{error:?}"
    );
    assert!(!has_call(&gh, "POST"), "nothing is created");
}

#[test]
fn portfolio_key_of_a_delivery_project_is_not_found() {
    let gh = base().ok("graphql", &projects("roadmap", "delivery"));
    let error = create_issues(&gh, &folder(), &options()).expect_err("not a portfolio");
    assert!(matches!(error, Error::Project(_)), "{error:?}");
    assert!(!has_call(&gh, "POST"), "nothing is created");
}

#[test]
fn portfolio_initiative_already_there_is_not_added() {
    let (draft, found) = draft_and_found();
    let gh = fake(&[item(11)]);
    let lines = add_to_portfolio(&gh, &draft, &found, &options()).expect("portfolio");
    assert!(lines.is_empty(), "{lines:?}");
    assert!(posts(&gh).is_empty(), "no POST is sent");
}

#[test]
fn portfolio_initiative_is_added_and_the_task_is_not() {
    let (draft, found) = draft_and_found();
    let gh = fake(&[item(5)]);
    let lines = add_to_portfolio(&gh, &draft, &found, &options()).expect("portfolio");
    assert_eq!(lines, ["portfolio: added #11"]);
    assert_eq!(
        posts(&gh),
        [json!({"type":"Issue","owner":"tailrocks","repo":"holla","number":11})]
    );
}

#[test]
fn portfolio_dry_run_names_each_initiative_and_makes_no_call() {
    let (draft, found) = draft_and_found();
    let gh = FakeGh::new();
    let lines = add_to_portfolio(&gh, &draft, &found, &dry_options()).expect("dry run");
    assert_eq!(lines, ["would add I1 to portfolio roadmap"]);
    assert!(gh.calls().is_empty(), "a dry run makes no call");
}
