//! Tests of the lookup of the existing issues.

use crate::apply::create_issues;

use super::super::tests::{base, created, folder, has_call, not_found, options, post_bodies};

#[test]
fn existing_markers_send_no_post() {
    let gh = not_found(
        base().ok(
            "issues?state=all",
            r#"[[{"number":21,"id":201,"body":"a\n\n<!-- taskrock:holla-csv:E1 -->"},{"number":22,"id":202,"body":"b\n\n<!-- taskrock:holla-csv:T01 -->"},{"number":23,"id":203,"body":"c\n\n<!-- taskrock:holla-csv:T02 -->"}]]"#,
        ),
        24,
    );
    let (_, map, lines) = create_issues(&gh, &folder(), &options()).expect("apply succeeds");
    assert!(!has_call(&gh, "POST"), "nothing is created: {lines:?}");
    assert_eq!(
        lines,
        [
            "exists  #21 E1 CSV export",
            "exists  #22 T01 Add CSV rendering to the report command",
            "exists  #23 T02 Add `--format csv` to the report command",
        ]
    );
    assert_eq!(map["T01"].link.number, 22);
    assert_eq!(map["T01"].id, 202);
}

#[test]
fn late_issue_after_empty_listing_creates_other_two() {
    let gh = created(
        not_found(
            base().ok("issues?state=all", "[[]]").ok(
                "holla/issues/1",
                r#"{"number":1,"id":50,"body":"late\n\n<!-- taskrock:holla-csv:T01 -->"}"#,
            ),
            2,
        ),
        11,
    );
    let (_, map, lines) = create_issues(&gh, &folder(), &options()).expect("apply succeeds");
    assert_eq!(
        lines,
        [
            "created #11 E1 CSV export",
            "exists  #1 T01 Add CSV rendering to the report command",
            "created #12 T02 Add `--format csv` to the report command",
        ]
    );
    assert_eq!(post_bodies(&gh).len(), 2, "T01 already exists");
    assert_eq!(map["T01"].id, 50, "the late issue is kept");
}

#[test]
fn three_missing_answers_end_the_look_past_the_listing() {
    let gh = created(
        not_found(
            base().ok(
                "issues?state=all",
                r#"[[{"number":5,"id":55,"body":"no marker"}]]"#,
            ),
            6,
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
    assert_eq!(post_bodies(&gh).len(), 3, "every issue is created");
    assert!(
        !has_call(&gh, "holla/issues/9"),
        "no fourth look past the listing"
    );
}
