//! Tests of the title, the privacy and the dry run.

use super::{DELIVERY, listing, node, one, run_step, sent};
use crate::gh::FakeGh;

#[test]
fn a_renamed_project_gets_its_title_back() {
    let found = node(9, "Old name", DELIVERY, false);
    let gh = FakeGh::new()
        .ok("projectsV2", &listing(&[found]))
        .ok("graphql", r#"{"data":{}}"#);
    let (result, lines) = run_step(&gh, &one(), false);
    result.expect("step");
    assert_eq!(lines, ["update project taskrock (title)"]);
    assert!(sent(&gh, 1).0.contains(r#"title:"taskrock""#));
}

#[test]
fn a_public_project_is_made_private() {
    let found = node(9, "taskrock", DELIVERY, true);
    let gh = FakeGh::new()
        .ok("projectsV2", &listing(&[found]))
        .ok("graphql", r#"{"data":{}}"#);
    let (result, lines) = run_step(&gh, &one(), false);
    let projects = result.expect("step");
    assert_eq!(lines, ["update project taskrock (private)"]);
    assert!(sent(&gh, 1).0.contains("public:false"));
    assert!(!projects[0].1.as_ref().expect("project").public);
}

#[test]
fn a_dry_run_with_a_missing_project_writes_nothing() {
    let gh = FakeGh::new().ok("projectsV2", &listing(&[]));
    let (result, lines) = run_step(&gh, &one(), true);
    let projects = result.expect("step");
    assert_eq!(
        lines,
        [
            "create project taskrock",
            "update project taskrock (marker)"
        ]
    );
    assert!(projects[0].1.is_none());
    assert_eq!(gh.calls().len(), 1);
}

#[test]
fn a_dry_run_with_a_renamed_project_writes_nothing() {
    let found = node(9, "Old name", DELIVERY, true);
    let gh = FakeGh::new().ok("projectsV2", &listing(&[found]));
    let (result, lines) = run_step(&gh, &one(), true);
    result.expect("step");
    assert_eq!(
        lines,
        [
            "update project taskrock (title)",
            "update project taskrock (private)"
        ]
    );
    assert_eq!(gh.calls().len(), 1);
}
