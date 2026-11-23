//! Tests of adding the issues to the project.

use serde_json::{Value, json};

use crate::apply::create_issues;
use crate::apply::project::add_to_project;
use crate::gh::FakeGh;

use super::super::tests::{base, dry_options, folder, fresh, has_call, not_found, options};

const FIELDS: &str = include_str!("../../../fixtures/project_fields.json");

/// One project item of the issue with `number` and database `id`.
fn item(number: u64, id: u64, title: &str) -> String {
    format!(
        "{{\"id\":{number},\"content_type\":\"Issue\",\"content\":{{\"html_url\":\"https://github.com/tailrocks/holla/issues/{number}\",\"id\":{id},\"title\":\"{title}\",\"state\":\"open\",\"type\":null,\"sub_issues_summary\":{{\"total\":0}},\"issue_dependencies_summary\":{{\"blocked_by\":0,\"blocking\":0}}}},\"fields\":[]}}"
    )
}

/// `fresh` with the project replies: the fields, the items of `first` and
/// then of `second`, and two accepted item `POST`s.
fn project_replies(gh: FakeGh, first: &str, second: &str) -> FakeGh {
    let fields = format!("[{FIELDS}]");
    gh.ok("POST orgs/tailrocks/projectsV2/7/items", "{}")
        .ok("POST orgs/tailrocks/projectsV2/7/items", "{}")
        .ok("7/fields", &fields)
        .ok("7/items?", &format!("[[{first}]]"))
        .ok("7/fields", &fields)
        .ok("7/items?", &format!("[[{second}]]"))
}

/// The inputs of the item `POST` calls, in call order.
fn item_posts(gh: &FakeGh) -> Vec<Value> {
    gh.calls()
        .into_iter()
        .filter(|(call, _)| call.contains("POST orgs/tailrocks/projectsV2/7/items"))
        .map(|(_, input)| {
            serde_json::from_str(&input.expect("a POST sends a body")).expect("input is JSON")
        })
        .collect()
}

/// Runs the creation and then the project step on the fresh draft with
/// these item pages. Returns the printed lines and the item `POST` bodies.
fn added(first: &str, second: &str) -> (Vec<String>, Vec<Value>) {
    let gh = project_replies(fresh(), first, second);
    let (draft, found, _) = create_issues(&gh, &folder(), &options()).expect("apply succeeds");
    let lines = add_to_project(&gh, &draft, &found, &options()).expect("project succeeds");
    (lines, item_posts(&gh))
}

#[test]
fn only_the_issue_with_no_parent_is_added() {
    let e1 = item(11, 111, "CSV export");
    let t01 = item(12, 112, "Add CSV rendering to the report command");
    let t02 = item(13, 113, "Add `--format csv` to the report command");
    let (lines, posts) = added(&format!("{t01},{t02}"), &format!("{e1},{t01},{t02}"));
    assert_eq!(lines, ["project: added #11"]);
    assert_eq!(
        posts,
        [json!({"type":"Issue","owner":"tailrocks","repo":"holla","number":11})]
    );
}

#[test]
fn second_read_with_every_issue_sends_no_further_post() {
    let e1 = item(11, 111, "CSV export");
    let t01 = item(12, 112, "Add CSV rendering to the report command");
    let t02 = item(13, 113, "Add `--format csv` to the report command");
    let (lines, posts) = added("", &format!("{e1},{t01},{t02}"));
    assert_eq!(lines, ["project: added #11"]);
    assert_eq!(posts.len(), 1, "no further POST is sent");
}

#[test]
fn issue_missing_after_last_read_is_added() {
    let e1 = item(11, 111, "CSV export");
    let t01 = item(12, 112, "Add CSV rendering to the report command");
    let (lines, posts) = added("", &format!("{e1},{t01}"));
    assert_eq!(lines, ["project: added #11", "project: added #13"]);
    assert_eq!(
        posts,
        [
            json!({"type":"Issue","owner":"tailrocks","repo":"holla","number":11}),
            json!({"type":"Issue","owner":"tailrocks","repo":"holla","number":13}),
        ]
    );
}

#[test]
fn dry_run_sends_no_post() {
    let gh = not_found(base().ok("issues?state=all", "[[]]"), 1);
    let (draft, found, _) =
        create_issues(&gh, &folder(), &dry_options()).expect("dry run succeeds");
    let lines = add_to_project(&gh, &draft, &found, &dry_options()).expect("dry run succeeds");
    assert!(!has_call(&gh, "POST"), "a dry run sends no POST");
    assert_eq!(lines, ["would add E1 to project"]);
}
