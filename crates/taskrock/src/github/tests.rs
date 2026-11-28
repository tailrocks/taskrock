//! Tests of the GitHub helpers.

use serde_json::json;

use crate::error::Error;
use crate::gh::FakeGh;
use crate::gh::Gh as _;
use crate::github::{get, get_all, graphql, is_status, send};

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

#[test]
fn graphql_returns_data() {
    let gh = FakeGh::new().ok("api graphql", r#"{"data":{"viewer":{"login":"a"}}}"#);
    let out = graphql(&gh, "query", &json!({"x": 1})).expect("data");
    assert_eq!(out, json!({"viewer": {"login": "a"}}));
    assert_eq!(
        gh.calls(),
        [(
            "api graphql --input -".to_owned(),
            Some(json!({"query": "query", "variables": {"x": 1}}).to_string())
        )]
    );
}

#[test]
fn graphql_turns_an_errors_answer_into_a_graphql_error() {
    let gh = FakeGh::new().ok(
        "api graphql",
        r#"{"data":null,"errors":[{"message":"one"},{"message":"two"}]}"#,
    );
    let error = graphql(&gh, "query", &json!({})).expect_err("errors");
    assert!(
        matches!(&error, Error::GraphQl { message } if message == "one; two"),
        "{error}"
    );
}

#[test]
fn graphql_turns_a_failed_gh_with_an_errors_answer_into_a_graphql_error() {
    let body = r#"{"errors":[{"message":"Field 'a' is wrong"},{"message":"Field 'b' is wrong"}]}"#;
    let gh = FakeGh::new().fail_with_output("api graphql", "gh: Field 'a' is wrong", body);
    let error = graphql(&gh, "query", &json!({})).expect_err("failed gh");
    let want = "Field 'a' is wrong; Field 'b' is wrong";
    assert!(
        matches!(&error, Error::GraphQl { message } if message == want),
        "{error}"
    );
}

#[test]
fn graphql_keeps_a_failure_that_is_not_an_answer_as_a_gh_error() {
    let body = r#"{"errors":[{"message":"Something went wrong"}]}"#;
    let cases = [
        ("gh: Bad Gateway (HTTP 502)", body),
        (
            "gh: Bad credentials (HTTP 401)",
            r#"{"message":"Bad credentials"}"#,
        ),
        ("Post \"https://api.github.com/graphql\": no such host", ""),
    ];
    for (stderr, stdout) in cases {
        let gh = FakeGh::new().fail_with_output("api graphql", stderr, stdout);
        let error = graphql(&gh, "query", &json!({})).expect_err("failed gh");
        assert!(
            matches!(&error, Error::Gh { stderr: seen, .. } if seen == stderr),
            "{error}"
        );
    }
}

#[test]
fn graphql_reads_a_message_that_mentions_http_on_a_later_line_as_an_answer() {
    let body =
        r#"{"errors":[{"message":"Field 'a' is wrong"},{"message":"Retry after HTTP 502"}]}"#;
    let stderr = "gh: Field 'a' is wrong\nRetry after HTTP 502";
    let gh = FakeGh::new().fail_with_output("api graphql", stderr, body);
    let error = graphql(&gh, "query", &json!({})).expect_err("failed gh");
    let want = "Field 'a' is wrong; Retry after HTTP 502";
    assert!(
        matches!(&error, Error::GraphQl { message } if message == want),
        "{error}"
    );
}
