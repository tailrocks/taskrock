//! Tests of the creation of the missing issues.

use serde_json::json;

use crate::apply::create_issues;

use super::super::tests::{
    base, dry_options, folder, fresh, has_call, not_found, options, post_bodies,
};

#[test]
fn first_run_creates_initiative_epic_then_tasks() {
    let gh = fresh();
    let (_, map, lines) = create_issues(&gh, &folder(), &options()).expect("apply succeeds");
    assert_eq!(
        lines,
        [
            "created #11 I1 CSV export everywhere",
            "created #12 E1 CSV export",
            "created #13 T01 Add CSV rendering to the report command",
            "created #14 T02 Add `--format csv` to the report command",
        ]
    );
    assert_eq!(map.len(), 4, "every key maps to an issue");
    let bodies = post_bodies(&gh);
    assert_eq!(bodies.len(), 4, "four issues are created");
    assert_eq!(bodies[0]["title"], json!("CSV export everywhere"));
    assert!(
        bodies[0].get("parent_issue_id").is_none(),
        "the initiative has no parent"
    );
    for body in &bodies[..2] {
        assert!(body.get("milestone").is_none(), "no milestone");
        assert!(body.get("issue_field_values").is_none(), "no routing keys");
    }
    assert_eq!(bodies[1]["parent_issue_id"], json!(111));
    assert_eq!(bodies[2]["parent_issue_id"], json!(112));
    let values = bodies[2]["issue_field_values"]
        .as_array()
        .expect("T01 has routing keys");
    assert_eq!(values.len(), 8, "T01 has eight routing keys");
    assert!(
        bodies[2]["body"]
            .as_str()
            .expect("T01 has a body")
            .ends_with("<!-- taskrock:holla-csv:T01 -->"),
        "the marker is the last line"
    );
    assert_eq!(bodies[3]["parent_issue_id"], json!(112));
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
            "would create I1 CSV export everywhere",
            "would create E1 CSV export",
            "would create T01 Add CSV rendering to the report command",
            "would create T02 Add `--format csv` to the report command",
        ]
    );
    assert!(map.is_empty(), "a dry run maps nothing: {map:?}");
}
