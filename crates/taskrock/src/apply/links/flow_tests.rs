//! Tests of `apply` on a draft whose issue links to existing issues and has a plan milestone.

use serde_json::json;

use crate::apply::blockers::link_blockers;
use crate::apply::create_issues;
use crate::gh::FakeGh;

use super::super::tests::{base, created, dry_options, has_call, not_found, options, post_bodies};
use super::tests::{FIVE, SIX, links_folder};

fn links_gh() -> FakeGh {
    not_found(base().ok("issues?state=all", "[[]]"), 1)
        .ok("repos/tailrocks/holla/issues/5", r#"{"number":5,"id":55}"#)
        .ok("repos/tailrocks/holla/issues/6", r#"{"number":6,"id":66}"#)
}

#[test]
fn issue_is_created_under_the_linked_parent() {
    let gh = created(links_gh(), 11);
    let (_, _, lines) = create_issues(&gh, &links_folder(), &options()).expect("apply succeeds");
    assert_eq!(lines.len(), 1, "{lines:?}");
    let bodies = post_bodies(&gh);
    assert_eq!(bodies[0]["parent_issue_id"], json!(55));
}

#[test]
fn issue_is_blocked_by_the_linked_issue_id() {
    let gh = created(links_gh(), 11)
        .ok("issues/11/dependencies/blocked_by?per_page", "[[]]")
        .ok(
            "POST repos/tailrocks/holla/issues/11/dependencies/blocked_by",
            "{}",
        );
    let (draft, found, _) = create_issues(&gh, &links_folder(), &options()).expect("apply");
    let lines = link_blockers(&gh, &draft, &found, false).expect("blockers");
    assert_eq!(lines, ["blocked #11 by #6"]);
    let sent: Vec<_> = gh
        .calls()
        .into_iter()
        .filter(|(call, _)| call.contains("POST") && call.contains("blocked_by"))
        .collect();
    assert_eq!(sent[0].1.as_deref(), Some(r#"{"issue_id":66}"#));
}

#[test]
fn dry_run_says_what_it_would_set() {
    let gh = links_gh();
    let (draft, found, lines) =
        create_issues(&gh, &links_folder(), &dry_options()).expect("dry run succeeds");
    assert_eq!(
        lines,
        [
            "would create L1 Add CSV rendering to the report command".to_owned(),
            format!("would set parent of L1 to {FIVE}"),
        ]
    );
    let blockers = link_blockers(&gh, &draft, &found, true).expect("dry blockers");
    assert_eq!(blockers, [format!("would block L1 by {SIX}")]);
    assert!(!has_call(&gh, "POST"), "a dry run sends no POST");
}
