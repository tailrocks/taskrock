//! Tests of the GitHub helpers.

use serde_json::json;

use crate::error::Error;
use crate::gh::FakeGh;
use crate::gh::Gh as _;
use crate::github::{get, get_all, is_status, send};

#[test]
fn get_parses_json() {
    let gh = FakeGh::new().ok("api", r#"{"a":1}"#);
    assert_eq!(get(&gh, "path").expect("json"), json!({"a": 1}));
}

#[test]
fn get_rejects_text_that_is_not_json() {
    let gh = FakeGh::new().ok("api", "nope");
    assert!(matches!(
        get(&gh, "path").expect_err("bad json"),
        Error::Json(_)
    ));
}

#[test]
fn get_all_joins_pages() {
    let gh = FakeGh::new().ok("api", "[[1,2],[3]]");
    assert_eq!(
        get_all(&gh, "path").expect("pages"),
        [json!(1), json!(2), json!(3)]
    );
}

#[test]
fn send_passes_body_as_input() {
    let gh = FakeGh::new().ok("api", r#"{"ok":true}"#);
    let body = json!({"a": 1});
    let out = send(&gh, "POST", "path", &body).expect("send");
    assert_eq!(out, json!({"ok": true}));
    assert_eq!(
        gh.calls(),
        [(
            "api -X POST path --input -".to_owned(),
            Some(body.to_string())
        )]
    );
}

#[test]
fn send_with_empty_output_returns_null() {
    let gh = FakeGh::new().ok("api", "");
    let out = send(&gh, "POST", "path", &json!({})).expect("empty");
    assert_eq!(out, serde_json::Value::Null);
}

#[test]
fn is_status_matches_http_code() {
    let gh = FakeGh::new().fail("api", "gh: Reference already exists (HTTP 422)");
    let error = gh.run(&["api"], None).expect_err("failed call");
    assert!(is_status(&error, 422));
    assert!(!is_status(&error, 404));
}
