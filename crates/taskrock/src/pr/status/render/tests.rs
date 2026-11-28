//! Tests of the text of a verdict.

use super::text;
use crate::config;
use crate::link::PullLink;
use crate::pr::snapshot::case_tests::Case;
use crate::pr::snapshot::{PullSnapshot, read};
use crate::pr::status::evaluate::{State, Verdict, evaluate};

pub(super) fn recorded(number: u64) -> (PullLink, PullSnapshot, Verdict) {
    let case = Case::recorded(number);
    let link = case.link();
    let snapshot = read(&case.gh(), &link).expect("the pull is read");
    let verdict = evaluate(&snapshot, &config::default(), &link);
    (link, snapshot, verdict)
}

#[test]
fn status_render_merged_has_one_line() {
    let (link, snapshot, verdict) = recorded(2932);
    assert_eq!(verdict.state, State::Merged);
    assert_eq!(
        text(&link, &snapshot, &verdict),
        format!(
            "tailrocks/velnor-new#2932: MERGED (head {}) -> merged as e9830e2dc6ab82db1e47e4ca909b4b53672ce2c2",
            &verdict.head[..7]
        )
    );
}

#[test]
fn status_render_ready_names_the_merge_command() {
    let (link, snapshot, mut verdict) = recorded(2932);
    verdict.state = State::Ready;
    verdict.next = format!("taskrock pr merge {}", link.url());
    let out = text(&link, &snapshot, &verdict);
    assert!(out.starts_with("tailrocks/velnor-new#2932: READY (head "));
    assert!(out.ends_with(&format!("-> taskrock pr merge {}", link.url())));
    assert_eq!(out.lines().count(), 1);
}

#[test]
fn status_render_waiting_marks_the_finding_as_a_wait() {
    let (link, snapshot, verdict) = recorded(3084);
    let out = text(&link, &snapshot, &verdict);
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines[0].starts_with("tailrocks/velnor-new#3084: WAITING (head "));
    assert!(lines[0].ends_with("-> wait for the merge queue"));
    assert_eq!(
        lines[1..],
        [
            "  [wait] queued (taskrock): The pull request is in the merge queue.",
            "      next: wait for the merge queue"
        ]
    );
}

#[test]
fn status_render_blocked_lists_each_blocker() {
    let (link, snapshot, verdict) = recorded(2963);
    let out = text(&link, &snapshot, &verdict);
    let heads: Vec<&str> = out.lines().filter(|l| l.starts_with("  [")).collect();
    assert!(out.starts_with("tailrocks/velnor-new#2963: BLOCKED (head "));
    assert_eq!(heads.len(), 3);
    assert!(heads[0].starts_with("  [blocker] conflict (builder): The branch conflicts with"));
    assert!(heads[1].starts_with("  [blocker] check_failed (builder): "));
    assert!(heads[2].starts_with("  [blocker] closes_missing (builder): "));
}

#[test]
fn status_render_caps_a_list_at_five_items() {
    let (link, snapshot, verdict) = recorded(3033);
    assert_eq!(verdict.findings[1].items.len(), 12);
    let out = text(&link, &snapshot, &verdict);
    let blocking = out
        .lines()
        .find(|l| l.contains("threads_blocking"))
        .expect("the blocking threads are listed");
    assert!(blocking.ends_with(", and 7 more"), "{blocking}");
    assert_eq!(blocking.matches("https://").count(), 5);
}

#[test]
fn status_render_does_not_cap_a_list_of_five() {
    let (link, snapshot, mut verdict) = recorded(3033);
    verdict.findings[1].items.truncate(5);
    let out = text(&link, &snapshot, &verdict);
    assert!(!out.contains(" more"));
}
