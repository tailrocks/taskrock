//! Tests of the milestone an issue takes from its link parent, and of the limit of sub-issues of a parent.

use serde_json::json;

use crate::apply::create_issues;
use crate::error::Error;
use crate::gh::FakeGh;

use super::super::tests::{
    FIELDS, MILESTONES, MISSING, created, dry_options, has_call, not_found, options, post_bodies,
};
use super::order_tests::{draft, task};
use super::tests::FIVE;

const TWO_MILESTONES: &str = r#"[[{"number":4,"title":"M1 Export"},{"number":5,"title":"M2"}]]"#;
const M2: &str = "milestone = \"M2\"";

/// The parent `FIVE`, with `extra` JSON members, and the `milestones` listing.
fn parented(milestones: &str, extra: &str) -> FakeGh {
    let parent = format!(r#"{{"number":5,"id":55{extra}}}"#);
    FakeGh::new()
        .fail("contents/taskrock.toml", MISSING)
        .ok("issue-fields", FIELDS)
        .ok("milestones?state=all", milestones)
        .ok("issues?state=all", "[[]]")
        .ok("repos/tailrocks/holla/issues/5", &parent)
}

fn inheriting(extra: &str) -> FakeGh {
    created(not_found(parented(MILESTONES, extra), 1), 11)
}

const M1_PARENT: &str = r#","milestone":{"title":"M1 Export"}"#;

/// Two tasks, `A1` with no milestone and `B1` with `M2`, in a plan with `plan`.
fn two_tasks(name: &str, plan: &str) -> std::path::PathBuf {
    let issues = [("A.md", task("A1", "")), ("B.md", task("B1", M2))];
    draft(name, plan, &issues)
}

#[test]
fn issue_takes_the_milestone_of_its_link_parent_unless_it_names_its_own() {
    let folder = two_tasks("inherit", "\n[[milestones]]\ntitle = \"M2\"\n");
    let gh = created(not_found(parented(TWO_MILESTONES, M1_PARENT), 1), 11);
    create_issues(&gh, &folder, &options()).expect("apply succeeds");
    let bodies = post_bodies(&gh);
    assert_eq!(bodies[0]["milestone"], json!(4), "the parent's milestone");
    assert_eq!(bodies[1]["milestone"], json!(5), "the issue's own wins");
}

#[test]
fn plan_milestone_wins_over_the_parent_milestone() {
    let folder = draft(
        "plan-wins",
        "milestone = \"M2\"\n",
        &[("A.md", task("A1", ""))],
    );
    let gh = created(not_found(parented(TWO_MILESTONES, M1_PARENT), 1), 11);
    create_issues(&gh, &folder, &options()).expect("apply succeeds");
    assert_eq!(post_bodies(&gh)[0]["milestone"], json!(5));
}

#[test]
fn parent_without_milestone_gives_the_issue_none() {
    let folder = draft("no-milestone", "", &[("A.md", task("A1", ""))]);
    let gh = inheriting("");
    create_issues(&gh, &folder, &options()).expect("apply succeeds");
    assert!(post_bodies(&gh)[0].get("milestone").is_none());
}

#[test]
fn inherited_milestone_missing_in_the_repository_stops_before_any_post() {
    let folder = draft("inherit-missing", "", &[("A.md", task("A1", ""))]);
    let gh = inheriting(r#","milestone":{"title":"M9"}"#);
    let Err(Error::Project(message)) = create_issues(&gh, &folder, &options()) else {
        panic!("expected Error::Project");
    };
    assert_eq!(
        message,
        "no milestone M9 in tailrocks/holla. Run taskrock setup"
    );
    assert!(
        !has_call(&gh, "POST"),
        "nothing is created, not even a milestone"
    );
}

#[test]
fn dry_run_names_the_parent_the_milestone_comes_from() {
    let folder = draft("inherit-dry", "", &[("A.md", task("A1", ""))]);
    let gh = not_found(parented(MILESTONES, M1_PARENT), 1);
    let (_, _, lines) = create_issues(&gh, &folder, &dry_options()).expect("dry run succeeds");
    let want = format!("would set milestone M1 Export on A1, from its parent {FIVE}");
    assert!(lines.contains(&want), "{lines:?}");
}

#[test]
fn parent_with_no_room_stops_before_any_post() {
    let folder = two_tasks("full", "\n[[milestones]]\ntitle = \"M2\"\n");
    let gh = created(
        not_found(
            parented(TWO_MILESTONES, r#","sub_issues_summary":{"total":100}"#),
            1,
        ),
        11,
    );
    let Err(Error::Apply(message)) = create_issues(&gh, &folder, &options()) else {
        panic!("expected Error::Apply");
    };
    assert_eq!(
        message,
        format!("{FIVE} has 100 sub-issues and GitHub allows 100: file under a child Epic of it")
    );
    assert!(!has_call(&gh, "POST"), "nothing is created");
}

#[test]
fn issues_already_created_do_not_count_against_the_limit_again() {
    let issues = [("A.md", task("A1", "")), ("B.md", task("B1", ""))];
    let folder = draft("rerun", "", &issues);
    let listing = r#"[[{"number":21,"id":201,"body":"x\n\n<!-- taskrock:holla-order:A1 -->"}]]"#;
    let gh = FakeGh::new()
        .fail("contents/taskrock.toml", MISSING)
        .ok("issue-fields", FIELDS)
        .ok("milestones?state=all", MILESTONES)
        .ok("issues?state=all", listing)
        .ok(
            "repos/tailrocks/holla/issues/5",
            r#"{"number":5,"id":55,"sub_issues_summary":{"total":99}}"#,
        );
    let gh = created(not_found(gh, 22), 11);
    let (_, _, lines) = create_issues(&gh, &folder, &options()).expect("99 and one new fit");
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_eq!(post_bodies(&gh).len(), 1, "only B1 is created");
}
