//! Tests for the bodies of ready tasks.

use std::collections::{BTreeMap, BTreeSet};

use super::{bodies, build_problems, named_files};
use crate::config::default;
use crate::fields::{ROUTING, Routing};
use crate::gh::FakeGh;
use crate::github::items::Item;
use crate::link::IssueLink;
use crate::ready::{Report, is_candidate, report};

fn link(number: u64) -> IssueLink {
    IssueLink {
        owner: "tailrocks".to_owned(),
        repo: "taskrock".to_owned(),
        number,
    }
}

fn numbered(number: u64) -> Item {
    Item {
        link: link(number),
        ..item("Task")
    }
}

fn listed<'a>(items: &[&'a Item], bodies: &BTreeMap<IssueLink, String>) -> Report<'a> {
    report(items, &BTreeSet::new(), bodies, &default())
}

fn item(kind: &str) -> Item {
    let mut routing = Routing::default();
    for field in ROUTING {
        routing.set(field, "x");
    }
    Item {
        item_id: 1,
        link: link(1),
        issue_id: 1,
        title: "T1".to_owned(),
        open: true,
        kind: Some(kind.to_owned()),
        children: 0,
        open_blockers: 0,
        blocking: 0,
        status: Some("Todo".to_owned()),
        parent: None,
        routing,
    }
}

const TASK: &str = "## Outcome\n\nx\n\n## Decisions already made\n\nNone.\n\n## Steps\n\n1. x\n\n## Done when\n\n- [ ] `true` passes.\n\n## Out of scope\n\nNone.\n";

#[test]
fn bodies_lists_open_issues_and_skips_pull_requests() {
    let answer = r#"[[{"number":1,"body":"one"},{"number":2,"body":"pr","pull_request":{}},{"number":3,"body":null}]]"#;
    let gh = FakeGh::new().ok(
        "repos/tailrocks/taskrock/issues?state=open&per_page=100",
        answer,
    );
    let task = item("Task");
    let found = bodies(&gh, &[&task, &task]).expect("bodies");
    assert_eq!(gh.calls().len(), 1, "one request for each repository");
    let numbers: Vec<(u64, &str)> = found.iter().map(|(k, v)| (k.number, v.as_str())).collect();
    assert_eq!(numbers, [(1, "one"), (3, "")]);
}

#[test]
fn a_complete_task_has_no_build_problems() {
    assert_eq!(
        build_problems(&item("Task"), TASK, &default()),
        [] as [&str; 0]
    );
}

#[test]
fn a_bug_in_todo_without_cause_has_a_problem() {
    let body = TASK.replace("## Outcome", "## What happens")
        + "\n## Expected\n\nx\n\n## How to reproduce\n\nx\n\n## Environment\n\nx\n";
    let found = build_problems(&item("Bug"), &body, &default());
    assert_eq!(found, ["the section ## Cause is missing"]);
}

#[test]
fn an_owner_task_uses_the_owner_format() {
    let mut owned = item("Task");
    owned.routing.set("Agent", "Owner");
    let found = build_problems(&owned, TASK, &default());
    assert!(
        found.iter().any(|line| line.contains("## Question")),
        "{found:?}"
    );
}

#[test]
fn named_files_takes_paths_from_decisions_only() {
    let body = "## Outcome\n\n`other/x.rs`\n\n## Decisions already made\n\n\
        Edit `crates/taskrock/src/ready.rs` and `src/ready/body.rs`, not `a b/c.rs`,\n\
        `Cargo.toml`, `src/ready` or `dir/`. Open `crates/taskrock/src/ready.rs` `unpaired/x.rs\n\
        ```rust\n/// `fenced/in.rs`\n```\n\n## Steps\n\n`later/y.rs`\n";
    let found: Vec<String> = named_files(body).into_iter().collect();
    assert_eq!(found, ["crates/taskrock/src/ready.rs", "src/ready/body.rs"]);
}

#[test]
fn a_body_that_breaks_its_format_is_not_ready() {
    let task = item("Task");
    let broken = BTreeMap::from([(link(1), "## Outcome\n\nx\n".to_owned())]);
    let found = listed(&[&task], &broken);
    assert_eq!(found.ready.len(), 0);
    assert_eq!(
        found.problems[0],
        "https://github.com/tailrocks/taskrock/issues/1 breaks the Task format: the section ## Decisions already made is missing"
    );
}

#[test]
fn a_feature_without_children_is_built_like_a_task() {
    let mut feature = item("Feature");
    let thin = BTreeMap::from([(link(1), "## Outcome\n".to_owned())]);
    let found = listed(&[&feature], &thin);
    assert_eq!(found.ready.len(), 0);
    assert!(found.problems[0].ends_with("Feature format: the section ## Problem is missing"));
    feature.children = 1;
    assert!(!is_candidate(&feature));
}

#[test]
fn two_ready_tasks_that_name_one_file_get_a_warning() {
    let (first, second, third) = (item("Task"), numbered(2), numbered(3));
    let named = TASK.replace(
        "None.\n\n## Steps",
        "Edit `crates/taskrock/src/ready.rs`.\n\n## Steps",
    );
    let mut bodies = BTreeMap::from([(link(3), TASK.to_owned())]);
    bodies.insert(link(1), named.clone());
    bodies.insert(link(2), named);
    let found = listed(&[&second, &first, &third], &bodies);
    assert_eq!(found.ready.len(), 3);
    assert_eq!(
        found.problems,
        [
            "crates/taskrock/src/ready.rs is named by https://github.com/tailrocks/taskrock/issues/1, https://github.com/tailrocks/taskrock/issues/2. Build them one after another"
        ]
    );
}
