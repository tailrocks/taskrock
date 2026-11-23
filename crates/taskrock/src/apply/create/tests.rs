//! Tests of the creation of the missing issues.

use serde_json::json;

use crate::apply::create_issues;

use super::super::tests::{
    base, created, dry_options, folder, has_call, not_found, options, post_bodies,
};

#[test]
fn first_run_creates_epic_then_tasks() {
    let gh = created(
        not_found(
            base().ok(
                "issues?state=all",
                r#"[[{"number":7,"id":70,"body":"old"},{"number":8,"id":80,"pull_request":{},"body":"old"}]]"#,
            ),
            9,
        ),
        11,
    );
    let (_, map, lines) = create_issues(&gh, &folder(), &options()).expect("apply succeeds");
    assert_eq!(
        lines,
        [
            "created #11 E1 CSV export",
            "created #12 T01 Add CSV rendering to the report command",
            "created #13 T02 Add `--format csv` to the report command",
        ]
    );
    assert_eq!(map.len(), 3, "every key maps to an issue");
    let bodies = post_bodies(&gh);
    assert_eq!(bodies.len(), 3, "three issues are created");
    assert_eq!(bodies[0]["title"], json!("CSV export"));
    assert!(
        bodies[0].get("parent_issue_id").is_none(),
        "the epic has no parent"
    );
    assert!(
        bodies[0].get("milestone").is_none(),
        "the epic has no milestone"
    );
    assert!(
        bodies[0].get("issue_field_values").is_none(),
        "the epic has no routing keys"
    );
    assert_eq!(bodies[1]["parent_issue_id"], json!(111));
    let values = bodies[1]["issue_field_values"]
        .as_array()
        .expect("T01 has routing keys");
    assert_eq!(values.len(), 8, "T01 has eight routing keys");
    assert!(
        bodies[1]["body"]
            .as_str()
            .expect("T01 has a body")
            .ends_with("<!-- taskrock:holla-csv:T01 -->"),
        "the marker is the last line"
    );
    assert_eq!(bodies[2]["parent_issue_id"], json!(111));
    assert!(!has_call(&gh, "POST repos/tailrocks/holla/milestones"));
}

#[test]
fn dry_run_sends_no_post() {
    let gh = not_found(base().ok("issues?state=all", "[[]]"), 1);
    let (_, map, lines) = create_issues(&gh, &folder(), &dry_options()).expect("dry run succeeds");
    assert!(!has_call(&gh, "POST"), "a dry run sends no POST");
    assert_eq!(
        lines,
        [
            "would create E1 CSV export",
            "would create T01 Add CSV rendering to the report command",
            "would create T02 Add `--format csv` to the report command",
        ]
    );
    assert!(map.is_empty(), "a dry run maps nothing: {map:?}");
}
