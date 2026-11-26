//! Tests of `apply`.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

use crate::apply::marker::{marker, marker_key};
use crate::apply::{Created, Options};
use crate::draft::{self, Draft};
use crate::gh::FakeGh;

pub(super) const FIELDS: &str = r#"[[{"id":11,"name":"Agent"},{"id":12,"name":"Runtime"},{"id":13,"name":"Model"},{"id":14,"name":"Effort"},{"id":15,"name":"Review agent"},{"id":16,"name":"Review runtime"},{"id":17,"name":"Review model"},{"id":18,"name":"Review effort"}]]"#;

pub(super) const MILESTONES: &str = r#"[[{"number":4,"title":"M1 Export"}]]"#;

pub(super) const MISSING: &str = "gh: Not Found (HTTP 404)";

pub(super) fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/draft-good")
}

/// The draft of `fixtures/<fixture>` with each `(key, number)` as issue
/// `number` of `tailrocks/holla`.
pub(super) fn draft_and_found(
    fixture: &str,
    issues: &[(&str, u64)],
) -> (Draft, BTreeMap<String, Created>) {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(fixture);
    let draft = draft::parse::read(&folder).expect("fixture is a draft");
    let mut found = BTreeMap::new();
    for (key, number) in issues {
        let link = format!("https://github.com/tailrocks/holla/issues/{number}");
        let link = link.parse().expect("link");
        found.insert(
            (*key).to_owned(),
            Created {
                link,
                id: number + 100,
            },
        );
    }
    (draft, found)
}

pub(super) fn options() -> Options {
    Options {
        dry_run: false,
        pause: Duration::from_secs(0),
        poll: Duration::from_secs(0),
    }
}

pub(super) fn dry_options() -> Options {
    Options {
        dry_run: true,
        pause: Duration::from_secs(0),
        poll: Duration::from_secs(0),
    }
}

pub(super) fn base() -> FakeGh {
    FakeGh::new()
        .fail("contents/taskrock.toml", MISSING)
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

/// The listing with two unrelated issues and nothing past it, then the
/// three issues of `draft-good` created with numbers 11 to 13.
pub(super) fn fresh() -> FakeGh {
    created(
        not_found(
            base().ok(
                "issues?state=all",
                r#"[[{"number":7,"id":70,"body":"old"},{"number":8,"id":80,"pull_request":{},"body":"old"}]]"#,
            ),
            9,
        ),
        11,
    )
}

/// The three issues of `draft-good` already on GitHub with numbers 21 to 23.
pub(super) fn all_existing() -> FakeGh {
    not_found(
        base().ok(
            "issues?state=all",
            r#"[[{"number":21,"id":201,"body":"a\n\n<!-- taskrock:holla-csv:E1 -->"},{"number":22,"id":202,"body":"b\n\n<!-- taskrock:holla-csv:T01 -->"},{"number":23,"id":203,"body":"c\n\n<!-- taskrock:holla-csv:T02 -->"}]]"#,
        ),
        24,
    )
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
