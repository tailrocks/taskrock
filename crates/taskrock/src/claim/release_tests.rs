//! Tests of `release`.

use crate::error::Error;
use crate::gh::FakeGh;

use super::release_at;

const URL: &str = "https://github.com/o/r/issues/7";
const LOOKUP: &str = r#"{"data":{"repository":{"issue":{"projectItems":{"nodes":[{"project":{"number":10,"owner":{"login":"o"}}}]}}}}}"#;
const FIELDS: &str = r#"[{"id":7,"name":"Status","options":[{"id":"t","name":{"raw":"Todo"}},{"id":"p","name":{"raw":"In Progress"}},{"id":"d","name":{"raw":"Done"}}]},{"id":9,"name":"Parent issue"}]"#;
const TASK: &str = r#"{"id":7,"content_type":"Issue","content":{"html_url":"https://github.com/o/r/issues/7","id":7,"title":"T","state":"open","type":{"name":"Task"},"sub_issues_summary":{"total":0},"issue_dependencies_summary":{"blocked_by":0,"blocking":0},"issue_field_values":[]},"fields":[{"name":"Status","value":{"name":{"raw":"Todo"}}},{"name":"Parent issue","value":{"url":"https://github.com/o/r/issues/5"}}]}"#;
const EPIC: &str = r#"{"id":5,"content_type":"Issue","content":{"html_url":"https://github.com/o/r/issues/5","id":5,"title":"E","state":"open","type":{"name":"Epic"},"sub_issues_summary":{"total":1},"issue_dependencies_summary":{"blocked_by":0,"blocking":0},"issue_field_values":[]},"fields":[{"name":"Status","value":{"name":{"raw":"Todo"}}},{"name":"Parent issue","value":null}]}"#;

fn pages(task: &str, rest: &[&str]) -> String {
    let mut all = vec![task.to_owned()];
    all.extend(rest.iter().map(|item| (*item).to_owned()));
    format!("[[{}]]", all.join(","))
}

fn base(items: &str) -> FakeGh {
    FakeGh::new()
        .ok("graphql", LOOKUP)
        .ok("fields", &format!("[{FIELDS}]"))
        .ok("items", items)
}

fn sibling(task: &str) -> String {
    task.replace("\"id\":7", "\"id\":8")
        .replace("issues/7", "issues/8")
}

fn with_parent(item: &str, number: u64) -> String {
    item.replace(
        "\"value\":null",
        &format!("\"value\":{{\"url\":\"https://github.com/o/r/issues/{number}\"}}"),
    )
}

fn present(gh: &FakeGh, what: &str) -> bool {
    gh.calls().into_iter().any(|(call, _)| call.contains(what))
}

fn patches(gh: &FakeGh) -> Vec<(String, Option<String>)> {
    gh.calls()
        .into_iter()
        .filter(|(call, _)| call.contains("PATCH"))
        .collect()
}

#[test]
fn release_without_branch_sets_todo() {
    let task = TASK.replace("Todo", "In Progress");
    let gh = base(&pages(&task, &[]))
        .fail("ref/heads", "gh: Not Found (HTTP 404)")
        .ok("fields", &format!("[{FIELDS}]"))
        .ok("PATCH", "");
    let out = release_at(&gh, URL, 99).expect("release succeeds");
    assert_eq!(out, "released https://github.com/o/r/issues/7\nno branch");
    assert_eq!(patches(&gh).len(), 1, "the task is set");
}

#[test]
fn release_with_merged_branch_deletes_it() {
    let task = TASK.replace("Todo", "In Progress");
    let gh = base(&pages(&task, &[]))
        .ok("ref/heads", r#"{"ref":"refs/heads/feat/issue-7"}"#)
        .ok("repos/o/r", r#"{"default_branch":"main"}"#)
        .ok("compare", r#"{"ahead_by":0,"behind_by":0}"#)
        .ok("DELETE", "")
        .ok("fields", &format!("[{FIELDS}]"))
        .ok("PATCH", "");
    let out = release_at(&gh, URL, 99).expect("release succeeds");
    assert!(out.ends_with("branch deleted"), "{out}");
    assert!(present(&gh, "DELETE"), "the branch is deleted");
}

#[test]
fn release_with_unmerged_branch_renames_it() {
    let task = TASK.replace("Todo", "In Progress");
    let gh = base(&pages(&task, &[]))
        .ok("ref/heads", r#"{"ref":"refs/heads/feat/issue-7"}"#)
        .ok("repos/o/r", r#"{"default_branch":"main"}"#)
        .ok("compare", r#"{"ahead_by":2,"behind_by":0}"#)
        .ok("rename", "{}")
        .ok("fields", &format!("[{FIELDS}]"))
        .ok("PATCH", "");
    let out = release_at(&gh, URL, 99).expect("release succeeds");
    assert!(
        out.ends_with("branch kept as feat/issue-7-parked-99"),
        "{out}"
    );
    assert!(!present(&gh, "DELETE"), "no delete is sent");
    let rename = gh
        .calls()
        .into_iter()
        .find(|(call, _)| call.contains("rename"))
        .expect("rename is sent");
    assert!(
        rename
            .1
            .as_ref()
            .is_some_and(|input| input.contains("feat/issue-7-parked-99")),
        "the fixed time names the branch: {rename:?}"
    );
}

#[test]
fn busy_parent_keeps_its_status() {
    let task = TASK.replace("Todo", "In Progress");
    let epic = EPIC.replace("Todo", "In Progress");
    let second = sibling(&TASK.replace("Todo", "In Progress"));
    let gh = base(&pages(&task, &[&epic, &second]))
        .fail("ref/heads", "gh: Not Found (HTTP 404)")
        .ok("fields", &format!("[{FIELDS}]"))
        .ok("PATCH", "");
    release_at(&gh, URL, 99).expect("release succeeds");
    let got = patches(&gh);
    assert_eq!(got.len(), 1, "only the task is set: {got:?}");
    assert!(got[0].0.contains("items/7"), "the task is set: {got:?}");
}

#[test]
fn free_chain_goes_back_to_todo() {
    let task = TASK.replace("Todo", "In Progress");
    let epic = with_parent(&EPIC.replace("Todo", "In Progress"), 6);
    let grand = EPIC
        .replace("\"id\":5", "\"id\":6")
        .replace("issues/5", "issues/6")
        .replace("Todo", "In Progress");
    let gh = base(&pages(&task, &[&epic, &grand]))
        .fail("ref/heads", "gh: Not Found (HTTP 404)")
        .ok("fields", &format!("[{FIELDS}]"))
        .ok("fields", &format!("[{FIELDS}]"))
        .ok("PATCH", "")
        .ok("PATCH", "")
        .ok("PATCH", "");
    release_at(&gh, URL, 99).expect("release succeeds");
    let got = patches(&gh);
    assert_eq!(got.len(), 3, "task, parent and grandparent: {got:?}");
    for item in ["items/7", "items/5", "items/6"] {
        assert!(
            got.iter().any(|(call, _)| call.contains(item)),
            "{item} is set"
        );
    }
}

#[test]
fn closed_issue_is_refused() {
    let task = TASK.replace("\"state\":\"open\"", "\"state\":\"closed\"");
    let gh = base(&pages(&task, &[]));
    let error = release_at(&gh, URL, 99).expect_err("release is refused");
    assert!(matches!(&error, Error::Refused(got) if got == "the issue is closed"));
    assert!(!present(&gh, "ref/heads"), "no branch call is made");
}
