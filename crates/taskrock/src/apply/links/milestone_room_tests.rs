//! Tests of the milestone that an issue keeps or takes in its own repository, and of the room under each parent.

use serde_json::{Value, json};

use crate::apply::create_issues;
use crate::error::Error;
use crate::gh::FakeGh;

use super::super::tests::{
    FIELDS, MILESTONES, MISSING, created, dry_options, has_call, not_found, options, post_bodies,
};
use super::milestone_tests::{M1_PARENT, TWO_MILESTONES, parented};
use super::order_tests::{draft, task};
use super::tests::{FIVE, SIX};

const M9_PARENT: &str = r#","milestone":{"title":"M9"}"#;

fn own_milestones() -> FakeGh {
    not_found(parented(TWO_MILESTONES, M9_PARENT), 1)
}

/// Applies `issues` under `plan`, the parent having a milestone the repository lacks.
/// Returns the milestone of each created issue.
fn applied(name: &str, plan: &str, issues: &[(&str, String)]) -> Vec<Value> {
    let folder = draft(name, plan, issues);
    let gh = created(own_milestones(), 11);
    create_issues(&gh, folder.path(), &options()).expect("apply succeeds");
    let (_, _, lines) =
        create_issues(&own_milestones(), folder.path(), &dry_options()).expect("dry run");
    assert!(
        lines.iter().all(|line| !line.contains("from its parent")),
        "{lines:?}"
    );
    post_bodies(&gh)
        .iter()
        .map(|body| body["milestone"].clone())
        .collect()
}

#[test]
fn own_milestone_is_kept_when_the_parent_milestone_is_missing_in_the_repo() {
    let own = [("A.md", task("A1", "milestone = \"M1 Export\""))];
    let plan = "\n[[milestones]]\ntitle = \"M1 Export\"\n";
    assert_eq!(applied("own-kept", plan, &own), [json!(4)]);
    let inherit = [("B.md", task("B1", ""))];
    let plan = "milestone = \"M2\"\n\n[[milestones]]\ntitle = \"M2\"\n";
    assert_eq!(applied("plan-kept", plan, &inherit), [json!(5)]);
}

#[test]
fn room_counts_only_the_issues_of_that_parent() {
    let other = format!("parent = \"{SIX}\"");
    let issues = [("A.md", task("A1", "")), ("B.md", task("B1", &other))];
    let folder = draft("room-per-parent", "", &issues);
    let full = parented(MILESTONES, r#","sub_issues_summary":{"total":99}"#)
        .ok("repos/tailrocks/holla/issues/6", r#"{"number":6,"id":66}"#);
    let gh = created(not_found(full, 1), 11);
    create_issues(&gh, folder.path(), &options()).expect("99 and one new fit under A");
    assert_eq!(post_bodies(&gh).len(), 2, "both issues are created");
}

#[test]
fn a_parent_at_100_refuses_one_more() {
    let folder = draft("full-by-one", "", &[("A.md", task("A1", ""))]);
    let full = parented(MILESTONES, r#","sub_issues_summary":{"total":100}"#);
    let gh = created(not_found(full, 1), 11);
    let Err(Error::Apply(message)) = create_issues(&gh, folder.path(), &options()) else {
        panic!("expected Error::Apply");
    };
    assert_eq!(
        message,
        format!("{FIVE} has 100 sub-issues and GitHub allows 100: file under a child Epic of it")
    );
    assert!(!has_call(&gh, "POST"), "nothing is created");
}

/// The `milestone` of the first issue `POST` to `repo`.
fn milestone_posted(gh: &FakeGh, repo: &str) -> Value {
    let post = format!("POST repos/{repo}/issues");
    let sent = gh
        .calls()
        .into_iter()
        .find(|(call, _)| call.contains(&post));
    let body = sent
        .and_then(|(_, input)| input)
        .expect("an issue is posted");
    serde_json::from_str::<Value>(&body).expect("input is JSON")["milestone"].clone()
}

#[test]
fn inherited_milestone_is_looked_up_in_the_repository_of_the_issue() {
    let other = format!("repo = \"tailrocks/other\"\nparent = \"{SIX}\"");
    let issues = [("A.md", task("A1", "")), ("B.md", task("B1", &other))];
    let folder = draft("two-repos", "", &issues);
    let parent_5 = format!(r#"{{"number":5,"id":55{M1_PARENT}}}"#);
    let parent_6 = r#"{"number":6,"id":66,"milestone":{"title":"M7"}}"#;
    let gh = FakeGh::new()
        .fail("contents/taskrock.toml", MISSING)
        .ok("issue-fields", FIELDS)
        .ok("milestones?state=all", MILESTONES)
        .ok("milestones?state=all", r#"[[{"number":9,"title":"M7"}]]"#)
        .ok("issues?state=all", "[[]]")
        .ok("issues?state=all", "[[]]")
        .ok("holla/issues/5", &parent_5)
        .ok("holla/issues/6", parent_6)
        .ok(
            "POST repos/tailrocks/holla/issues",
            r#"{"number":11,"id":111}"#,
        )
        .ok(
            "POST repos/tailrocks/other/issues",
            r#"{"number":1,"id":301}"#,
        );
    let gh = not_found(gh, 1)
        .fail("other/issues/1", MISSING)
        .fail("other/issues/2", MISSING)
        .fail("other/issues/3", MISSING);
    create_issues(&gh, folder.path(), &options()).expect("apply succeeds");
    assert_eq!(milestone_posted(&gh, "tailrocks/holla"), json!(4));
    assert_eq!(milestone_posted(&gh, "tailrocks/other"), json!(9));
}
