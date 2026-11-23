//! Tests of `claim`.

use crate::error::Error;
use crate::gh::FakeGh;

use super::claim;

const URL: &str = "https://github.com/o/r/issues/7";
const TAKEN: &str = "the task is already taken: the branch feat/issue-7 exists";
const KIDS: &str = "the issue has children. Only an issue with no children is built";
const AGENT: &str = r#"{"issue_field_name":"Agent","single_select_option":{"name":"Owner"}}"#;
const LOOKUP: &str = r#"{"data":{"repository":{"issue":{"projectItems":{"nodes":[{"project":{"number":10,"owner":{"login":"o"}}}]}}}}}"#;
const FIELDS: &str = r#"[{"id":7,"name":"Status","options":[{"id":"t","name":{"raw":"Todo"}},{"id":"p","name":{"raw":"In Progress"}},{"id":"d","name":{"raw":"Done"}}]},{"id":9,"name":"Parent issue"}]"#;
const TASK: &str = r#"{"id":7,"content_type":"Issue","content":{"html_url":"https://github.com/o/r/issues/7","id":7,"title":"T","state":"open","type":{"name":"Task"},"sub_issues_summary":{"total":0},"issue_dependencies_summary":{"blocked_by":0,"blocking":0},"issue_field_values":[]},"fields":[{"name":"Status","value":{"name":{"raw":"Todo"}}},{"name":"Parent issue","value":{"url":"https://github.com/o/r/issues/5"}}]}"#;
const EPIC: &str = r#"{"id":5,"content_type":"Issue","content":{"html_url":"https://github.com/o/r/issues/5","id":5,"title":"E","state":"open","type":{"name":"Epic"},"sub_issues_summary":{"total":1},"issue_dependencies_summary":{"blocked_by":0,"blocking":0},"issue_field_values":[]},"fields":[{"name":"Status","value":{"name":{"raw":"Todo"}}},{"name":"Parent issue","value":null}]}"#;

fn pages(task: &str, epic: Option<&str>) -> String {
    match epic {
        Some(epic) => format!("[[{task},{epic}]]"),
        None => format!("[[{task}]]"),
    }
}

fn base(items: &str) -> FakeGh {
    FakeGh::new()
        .ok("graphql", LOOKUP)
        .ok("fields", &format!("[{FIELDS}]"))
        .ok("items", items)
        .ok("repos/o/r", r#"{"default_branch":"main"}"#)
        .ok("ref/heads", r#"{"object":{"sha":"abc"}}"#)
}

fn full(task: &str, epic: Option<&str>) -> FakeGh {
    let gh = base(&pages(task, epic)).ok("POST", "{}");
    gh.ok("fields", &format!("[{FIELDS}]"))
        .ok("PATCH", "")
        .ok("PATCH", "")
}

fn refused(bad: &str, message: &str) {
    let gh = full(bad, None);
    let error = claim(&gh, URL).expect_err("claim is refused");
    assert!(matches!(&error, Error::Refused(got) if got == message));
    assert!(!has(&gh, "POST"));
}

fn has(gh: &FakeGh, what: &str) -> bool {
    gh.calls().into_iter().any(|(call, _)| call.contains(what))
}

#[test]
fn good_claim_sets_task_and_parent() {
    let gh = full(TASK, Some(EPIC));
    let out = claim(&gh, URL).expect("claim succeeds");
    assert_eq!(
        out,
        "taken https://github.com/o/r/issues/7\nbranch feat/issue-7"
    );
    let calls: Vec<String> = gh.calls().into_iter().map(|(call, _)| call).collect();
    assert_eq!(calls.len(), 9, "nine calls: {calls:?}");
    let want = "graphql fields items repo heads POST fields items/7 items/5".split(' ');
    for (got, marker) in calls.iter().zip(want) {
        assert!(got.contains(marker), "{got} holds {marker}");
    }
}

#[test]
fn closed_issue_is_refused() {
    refused(&TASK.replace("open", "closed"), "the issue is closed");
}

#[test]
fn epic_is_refused() {
    refused(
        &TASK.replace("Task", "Epic"),
        "only a Task or a Bug can be taken",
    );
}

#[test]
fn parent_issue_is_refused() {
    refused(&TASK.replace("\"total\":0", "\"total\":2"), KIDS);
}

#[test]
fn owner_issue_is_refused() {
    refused(
        &TASK.replace("\":[]", &format!("\":[{AGENT}]")),
        "this is the owner's task",
    );
}

#[test]
fn blocked_issue_is_refused() {
    refused(
        &TASK.replace("\"blocked_by\":0", "\"blocked_by\":1"),
        "1 open issues block it",
    );
}

#[test]
fn done_issue_is_refused() {
    refused(
        &TASK.replace("Todo", "Done"),
        "the status is Done, not Todo",
    );
}

#[test]
fn missing_status_is_refused() {
    refused(
        &TASK.replace(":{\"name\":{\"raw\":\"Todo\"}}", ":null"),
        "the issue has no status",
    );
}

#[test]
fn taken_branch_is_refused() {
    let gh = base(&pages(TASK, None)).fail("POST", "gh: Reference already exists (HTTP 422)");
    let error = claim(&gh, URL).expect_err("branch is taken");
    assert!(matches!(&error, Error::Refused(got) if got == TAKEN));
    assert!(!has(&gh, "PATCH"));
}

#[test]
fn failed_status_deletes_branch() {
    let gh = base(&pages(TASK, None))
        .ok("POST", "{}")
        .ok("fields", &format!("[{FIELDS}]"));
    let gh = gh.fail("PATCH", "boom").ok("DELETE", "");
    let error = claim(&gh, URL).expect_err("status fails");
    assert!(matches!(&error, Error::Gh { .. }));
    assert!(has(&gh, "DELETE"));
}

#[test]
fn in_progress_parent_gets_no_status() {
    let epic = EPIC.replace("Todo", "In Progress");
    let gh = full(TASK, Some(&epic));
    claim(&gh, URL).expect("claim succeeds");
    let patches = gh
        .calls()
        .iter()
        .filter(|(c, _)| c.contains("PATCH"))
        .count();
    assert_eq!(patches, 1, "only the task is set");
}
