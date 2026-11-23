//! Tests of `apply`.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

use crate::apply::Options;
use crate::apply::marker::{marker, marker_key};
use crate::gh::FakeGh;

pub(super) const FIELDS: &str = r#"[[{"id":11,"name":"Agent"},{"id":12,"name":"Runtime"},{"id":13,"name":"Model"},{"id":14,"name":"Effort"},{"id":15,"name":"Review agent"},{"id":16,"name":"Review runtime"},{"id":17,"name":"Review model"},{"id":18,"name":"Review effort"}]]"#;

pub(super) const MILESTONES: &str = r#"[[{"number":4,"title":"M1 Export"}]]"#;

pub(super) const MISSING: &str = "gh: Not Found (HTTP 404)";

pub(super) fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/draft-good")
}

pub(super) fn options() -> Options {
    Options {
        dry_run: false,
        pause: Duration::from_secs(0),
    }
}

pub(super) fn dry_options() -> Options {
    Options {
        dry_run: true,
        pause: Duration::from_secs(0),
    }
}

pub(super) fn base() -> FakeGh {
    FakeGh::new()
        .ok("issue-fields", FIELDS)
        .ok("milestones?state=all", MILESTONES)
}

pub(super) fn not_found(gh: FakeGh, first: u64) -> FakeGh {
    let mut gh = gh;
    for number in first..first + 3 {
        gh = gh.fail(&format!("holla/issues/{number}"), MISSING);
    }
    gh
}

pub(super) fn created(gh: FakeGh, first: u64) -> FakeGh {
    let mut gh = gh;
    for number in first..first + 3 {
        let id = 100 + number;
        gh = gh.ok(
            "POST repos/tailrocks/holla/issues",
            &format!(r#"{{"number":{number},"id":{id}}}"#),
        );
    }
    gh
}

/// The inputs of the issue `POST` calls, in call order.
pub(super) fn post_bodies(gh: &FakeGh) -> Vec<Value> {
    gh.calls()
        .into_iter()
        .filter(|(call, _)| call.contains("POST repos/tailrocks/holla/issues"))
        .map(|(_, input)| {
            serde_json::from_str(&input.expect("a POST sends a body")).expect("input is JSON")
        })
        .collect()
}

pub(super) fn has_call(gh: &FakeGh, what: &str) -> bool {
    gh.calls().iter().any(|(call, _)| call.contains(what))
}

#[test]
fn marker_names_plan_and_key() {
    assert_eq!(marker("plan", "T01"), "<!-- taskrock:plan:T01 -->");
    assert_eq!(
        marker_key("body\n\n<!-- taskrock:plan:T01 -->", "plan"),
        Some("T01".to_owned())
    );
    assert_eq!(marker_key("no marker", "plan"), None);
    assert_eq!(
        marker_key("<!-- taskrock:other:T01 -->", "plan"),
        None,
        "another plan is ignored"
    );
}
