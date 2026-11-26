//! Tests of the milestones step.

use serde_json::json;

use super::milestones;
use crate::config::SharedMilestone;
use crate::gh::FakeGh;
use crate::setup::testing::{body, project, step};

fn wanted(due: Option<&str>, description: Option<&str>) -> Vec<SharedMilestone> {
    vec![SharedMilestone {
        title: "v0.2.0".to_owned(),
        due: due.map(str::to_owned),
        description: description.map(str::to_owned),
    }]
}

fn present(due_on: Option<&str>, description: Option<&str>) -> String {
    json!([
        {"number": 1, "title": "other", "description": null, "due_on": null},
        {"number": 7, "title": "v0.2.0", "description": description, "due_on": due_on},
    ])
    .to_string()
}

#[test]
fn a_missing_milestone_is_created_with_due_on_at_noon() {
    let gh = FakeGh::new()
        .ok("milestones?state=all&per_page=100", "[]")
        .ok("-X POST repos/acme/taskrock/milestones", "{}");
    let list = [project(
        &["taskrock"],
        wanted(Some("2026-12-15"), Some("The release")),
        true,
    )];
    let (result, lines) = step(&gh, false, |run| milestones(run, &list));
    result.expect("step");
    assert_eq!(lines, ["create milestone taskrock v0.2.0"]);
    assert_eq!(
        body(&gh, 1),
        json!({"title": "v0.2.0", "description": "The release", "due_on": "2026-12-15T12:00:00Z"})
    );
}

#[test]
fn a_milestone_with_only_a_title_sends_only_the_title() {
    let gh = FakeGh::new()
        .ok("milestones?state=all", "[]")
        .ok("-X POST", "{}");
    let list = [project(&["taskrock"], wanted(None, None), true)];
    step(&gh, false, |run| milestones(run, &list))
        .0
        .expect("step");
    assert_eq!(body(&gh, 1), json!({"title": "v0.2.0"}));
}

#[test]
fn another_due_date_is_fixed_with_a_patch() {
    let gh = FakeGh::new()
        .ok(
            "milestones?state=all",
            &present(Some("2026-12-01T00:00:00Z"), None),
        )
        .ok("-X PATCH repos/acme/taskrock/milestones/7", "{}");
    let list = [project(
        &["taskrock"],
        wanted(Some("2026-12-15"), None),
        true,
    )];
    let (result, lines) = step(&gh, false, |run| milestones(run, &list));
    result.expect("step");
    assert_eq!(lines, ["update milestone taskrock v0.2.0"]);
    assert_eq!(body(&gh, 1), json!({"due_on": "2026-12-15T12:00:00Z"}));
}

#[test]
fn another_description_is_fixed_with_a_patch() {
    let gh = FakeGh::new()
        .ok(
            "milestones?state=all",
            &present(Some("2026-12-15T00:00:00Z"), Some("old")),
        )
        .ok("-X PATCH", "{}");
    let list = [project(
        &["taskrock"],
        wanted(Some("2026-12-15"), Some("new")),
        true,
    )];
    let (result, lines) = step(&gh, false, |run| milestones(run, &list));
    result.expect("step");
    assert_eq!(lines, ["update milestone taskrock v0.2.0"]);
    assert_eq!(body(&gh, 1)["description"], "new");
}

#[test]
fn nothing_to_do_when_the_milestone_matches() {
    let gh = FakeGh::new().ok(
        "milestones?state=all",
        &present(Some("2026-12-15T00:00:00Z"), Some("The release")),
    );
    let list = [project(
        &["taskrock"],
        wanted(Some("2026-12-15"), Some("The release")),
        true,
    )];
    let (result, lines) = step(&gh, false, |run| milestones(run, &list));
    result.expect("step");
    assert_eq!(lines, ["exists milestone taskrock v0.2.0"]);
    assert_eq!(gh.calls().len(), 1);
}

#[test]
fn a_dry_run_writes_nothing() {
    let gh = FakeGh::new().ok(
        "milestones?state=all",
        &present(Some("2026-12-01T00:00:00Z"), None),
    );
    let list = [project(
        &["taskrock"],
        wanted(Some("2026-12-15"), None),
        true,
    )];
    let (result, lines) = step(&gh, true, |run| milestones(run, &list));
    result.expect("step");
    assert_eq!(lines, ["update milestone taskrock v0.2.0"]);
    assert_eq!(gh.calls().len(), 1);
}

#[test]
fn a_project_that_is_none_says_create_in_each_repository_with_no_read() {
    let gh = FakeGh::new();
    let list = [project(&["a", "b"], wanted(None, None), false)];
    let (result, lines) = step(&gh, true, |run| milestones(run, &list));
    result.expect("step");
    assert_eq!(
        lines,
        ["create milestone a v0.2.0", "create milestone b v0.2.0"]
    );
    assert_eq!(gh.calls().len(), 0);
}

#[test]
fn a_project_without_milestones_reads_nothing() {
    let gh = FakeGh::new();
    let list = [project(&["taskrock"], Vec::new(), true)];
    step(&gh, false, |run| milestones(run, &list))
        .0
        .expect("step");
    assert_eq!(gh.calls().len(), 0);
}
