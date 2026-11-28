//! Tests of `base`.

use crate::error::Error;
use crate::gh::FakeGh;
use crate::github::items::Item;
use crate::link::IssueLink;

use super::base;

fn link(number: u64) -> IssueLink {
    IssueLink {
        owner: "o".to_owned(),
        repo: "r".to_owned(),
        number,
    }
}

fn item(number: u64, status: &str) -> Item {
    Item {
        item_id: number,
        link: link(number),
        issue_id: number,
        title: "T".to_owned(),
        open: true,
        kind: Some("Task".to_owned()),
        children: 0,
        open_blockers: 0,
        blocking: 0,
        status: Some(status.to_owned()),
        parent: None,
        milestone: None,
        routing: crate::fields::Routing::default(),
    }
}

fn blockers(numbers: &[(&str, u64)]) -> String {
    let list: Vec<String> = numbers
        .iter()
        .map(|(repo, number)| {
            format!(
                r#"{{"state":"open","html_url":"https://github.com/o/{repo}/issues/{number}"}}"#
            )
        })
        .collect();
    format!("[[{}]]", list.join(","))
}

fn pull(number: u64, base: &str) -> String {
    format!(
        r#"[[{{"html_url":"https://github.com/o/r/pull/{}","base":{{"ref":"{base}"}}}}]]"#,
        number + 100
    )
}

fn heads(number: u64) -> String {
    format!("head=o:feat/issue-{number}&")
}

fn refused(gh: &FakeGh, items: &[Item]) -> String {
    let error = base(gh, items, &item(7, "Todo")).expect_err("the stack is refused");
    match error {
        Error::Refused(message) => message,
        other => panic!("not a refusal: {other}"),
    }
}

#[test]
fn one_blocker_with_an_open_pull_request_is_the_base() {
    let gh = FakeGh::new()
        .ok("blocked_by", &blockers(&[("r", 5)]))
        .ok(&heads(5), &pull(5, "main"));
    let stack = base(&gh, &[item(5, "In Progress")], &item(7, "Todo")).expect("a stack");
    assert_eq!(stack.blocker, link(5));
    assert_eq!(stack.base, "feat/issue-5");
    assert_eq!(stack.pull_request, "https://github.com/o/r/pull/105");
}

#[test]
fn a_blocker_in_todo_is_refused() {
    let gh = FakeGh::new().ok("blocked_by", &blockers(&[("r", 5)]));
    assert_eq!(
        refused(&gh, &[item(5, "Todo")]),
        "cannot stack on https://github.com/o/r/issues/5: its status is Todo, not In Progress"
    );
    assert_eq!(
        refused(&FakeGh::new().ok("blocked_by", &blockers(&[("r", 5)])), &[]),
        "cannot stack on https://github.com/o/r/issues/5: its status is none, not In Progress"
    );
    let closed = r#"[[{"state":"closed","html_url":"https://github.com/o/r/issues/5"}]]"#;
    assert_eq!(
        refused(&FakeGh::new().ok("blocked_by", closed), &[]),
        "cannot stack: the task has no open blocker"
    );
}

#[test]
fn a_blocker_without_a_pull_request_is_refused() {
    let gh = FakeGh::new()
        .ok("blocked_by", &blockers(&[("r", 5)]))
        .ok(&heads(5), "[[]]");
    assert_eq!(
        refused(&gh, &[item(5, "In Progress")]),
        "cannot stack on https://github.com/o/r/issues/5: feat/issue-5 has no open pull request"
    );
}

#[test]
fn a_blocker_in_another_repository_is_refused() {
    let gh = FakeGh::new().ok("blocked_by", &blockers(&[("other", 5)]));
    assert_eq!(
        refused(&gh, &[item(5, "In Progress")]),
        "cannot stack on https://github.com/o/other/issues/5: it is in another repository"
    );
}

#[test]
fn two_blockers_in_one_stack_take_the_top() {
    let gh = FakeGh::new()
        .ok("blocked_by", &blockers(&[("r", 5), ("r", 6)]))
        .ok(&heads(5), &pull(5, "main"))
        .ok(&heads(6), &pull(6, "feat/issue-5"))
        .ok(&heads(5), &pull(5, "main"));
    let items = [item(5, "In Progress"), item(6, "In Progress")];
    let stack = base(&gh, &items, &item(7, "Todo")).expect("a stack");
    assert_eq!(stack.base, "feat/issue-6");
    assert_eq!(stack.pull_request, "https://github.com/o/r/pull/106");
}

#[test]
fn two_blockers_in_two_stacks_are_refused() {
    let gh = FakeGh::new()
        .ok("blocked_by", &blockers(&[("r", 5), ("r", 6)]))
        .ok(&heads(5), &pull(5, "main"))
        .ok(&heads(6), &pull(6, "main"));
    let items = [item(5, "In Progress"), item(6, "In Progress")];
    assert_eq!(
        refused(&gh, &items),
        "cannot stack: the open blockers https://github.com/o/r/issues/5, https://github.com/o/r/issues/6 are not one stack. Build it after all but one of them close"
    );
}
