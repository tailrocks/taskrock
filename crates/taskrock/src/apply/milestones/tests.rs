//! Tests of the milestone numbers.

use serde_json::{Value, json};

use crate::apply::create_issues;
use crate::gh::FakeGh;

use super::super::tests::{FIELDS, MISSING, created, folder, not_found, options, post_bodies};

#[test]
fn missing_milestone_is_created_once() {
    let gh = created(
        not_found(
            FakeGh::new()
                .fail("contents/taskrock.toml", MISSING)
                .ok("issue-fields", FIELDS)
                .ok("milestones?state=all", "[[]]")
                .ok(
                    "POST repos/tailrocks/holla/milestones",
                    r#"{"number":4,"title":"M1 Export"}"#,
                )
                .ok("issues?state=all", "[[]]"),
            1,
        ),
        11,
    );
    let (_, _, lines) = create_issues(&gh, &folder(), &options()).expect("apply succeeds");
    assert_eq!(
        lines,
        [
            "created #11 E1 CSV export",
            "created #12 T01 Add CSV rendering to the report command",
            "created #13 T02 Add `--format csv` to the report command",
        ]
    );
    let milestone_posts: Vec<_> = gh
        .calls()
        .into_iter()
        .filter(|(call, _)| call.contains("POST repos/tailrocks/holla/milestones"))
        .collect();
    assert_eq!(milestone_posts.len(), 1, "the milestone is created once");
    let sent: Value = serde_json::from_str(
        milestone_posts[0]
            .1
            .as_deref()
            .expect("a milestone POST sends a body"),
    )
    .expect("milestone input is JSON");
    assert_eq!(
        sent,
        json!({"title": "M1 Export", "description": "CSV export works for the summary report."})
    );
    for body in post_bodies(&gh).into_iter().skip(1) {
        assert_eq!(body["milestone"], json!(4), "tasks reuse the milestone");
    }
}
