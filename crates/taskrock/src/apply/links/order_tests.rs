//! Tests of the order of the reads and writes of `apply`: nothing is created before the draft is known to apply.

use std::path::PathBuf;

use crate::apply::create_issues;
use crate::error::Error;
use crate::gh::FakeGh;

use super::super::tests::{FIELDS, MISSING, created, dry_options, has_call, not_found, options};

const PLAN: &str = "id = \"holla-order\"\nrepo = \"tailrocks/holla\"\nproject = \"https://github.com/orgs/tailrocks/projects/7\"\n";

/// The Task `L1` of `draft-links` with `front` in place of its `parent` and `blocked_by`.
fn task(key: &str, front: &str) -> String {
    let path = super::tests::links_folder().join("L1.md");
    let text = std::fs::read_to_string(path).expect("fixture reads");
    let text = text
        .lines()
        .filter(|line| !line.starts_with("parent") && !line.starts_with("blocked_by"))
        .collect::<Vec<_>>()
        .join("\n");
    text.replacen("key = \"L1\"", &format!("key = \"{key}\"\n{front}"), 1)
}

/// A draft folder in the temporary directory with `plan` and these `(file, text)` issues.
fn draft(name: &str, plan: &str, issues: &[(&str, String)]) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("taskrock-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&folder).expect("folder is made");
    std::fs::write(folder.join("plan.toml"), format!("{PLAN}{plan}")).expect("plan is written");
    for (file, text) in issues {
        std::fs::write(folder.join(file), text).expect("issue is written");
    }
    folder
}

fn gh(milestones: &[&str]) -> FakeGh {
    let mut gh = FakeGh::new()
        .fail("contents/taskrock.toml", MISSING)
        .ok("issue-fields", FIELDS);
    for listing in milestones {
        gh = gh.ok("milestones?state=all", listing);
    }
    gh.ok("issues?state=all", "[[]]")
}

const M2: &str = "\n[[milestones]]\ntitle = \"M2\"\n";

#[test]
fn unknown_link_stops_before_any_milestone_post() {
    let own = "milestone = \"M2\"\nparent = \"https://github.com/tailrocks/holla/issues/5\"";
    let folder = draft("link-first", M2, &[("L1.md", task("L1", own))]);
    let gh = gh(&["[[]]"]).fail("holla/issues/5", MISSING);
    let Err(Error::Apply(message)) = create_issues(&gh, &folder, &options()) else {
        panic!("expected Error::Apply");
    };
    assert_eq!(
        message,
        "no issue https://github.com/tailrocks/holla/issues/5"
    );
    assert!(!has_call(&gh, "POST"), "no milestone is created");
}

#[test]
fn plan_milestone_is_checked_in_every_repo_before_any_create() {
    let plan = format!("milestone = \"M9\"\n{M2}");
    let issues = [
        ("A.md", task("A1", "milestone = \"M2\"")),
        ("B.md", task("B1", "repo = \"tailrocks/other\"")),
    ];
    let folder = draft("every-repo", &plan, &issues);
    let gh = gh(&["[[]]", "[[]]"]);
    let Err(Error::Project(message)) = create_issues(&gh, &folder, &options()) else {
        panic!("expected Error::Project");
    };
    assert_eq!(
        message,
        "no milestone M9 in tailrocks/other. Run taskrock setup"
    );
    assert!(
        !has_call(&gh, "POST"),
        "no milestone and no issue is created"
    );
}

#[test]
fn dry_run_and_real_run_agree_on_a_plan_milestone_that_apply_creates() {
    let plan = format!("milestone = \"M2\"\n{M2}");
    let issues = [
        ("A.md", task("A1", "milestone = \"M2\"")),
        ("B.md", task("B1", "")),
    ];
    let folder = draft("agree", &plan, &issues);
    let dry = not_found(gh(&["[[]]"]), 1);
    let (_, _, lines) = create_issues(&dry, &folder, &dry_options()).expect("dry run succeeds");
    assert!(
        lines.contains(&"would set milestone M2 on B1".to_owned()),
        "{lines:?}"
    );
    let real = created(
        not_found(
            gh(&["[[]]"]).ok(
                "POST repos/tailrocks/holla/milestones",
                r#"{"number":4,"title":"M2"}"#,
            ),
            1,
        ),
        11,
    );
    create_issues(&real, &folder, &options()).expect("real run succeeds");
    assert!(has_call(&real, "POST repos/tailrocks/holla/milestones"));
}
