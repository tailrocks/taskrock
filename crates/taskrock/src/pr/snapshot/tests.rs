//! Tests of the snapshot, from the recorded pull requests of tailrocks/velnor-new.

use super::case::{Case, page};
use super::rest::verdict;
use super::{PullState, read};
use crate::gh::FakeGh;

fn snapshot(case: &Case) -> super::PullSnapshot {
    read(&case.gh(), &case.link()).expect("the pull is read")
}

#[test]
fn snapshot_reads_a_merged_pull_in_full() {
    let snapshot = snapshot(&Case::recorded(2932));
    assert_eq!(snapshot.state, PullState::Merged);
    assert!(snapshot.merge_commit.is_some() && !snapshot.cross_repository);
    assert_eq!((snapshot.checks.len(), snapshot.threads.len()), (18, 76));
    assert_eq!(snapshot.required_checks, ["Required"]);
    assert!(snapshot.queue.has_queue && snapshot.queue.entry.is_none());
    let last = snapshot.verdicts.last().expect("a verdict");
    assert!(last.pass && last.head == snapshot.head_sha);
    assert_eq!(snapshot.codex.expect("codex").code_status, "completed");
    assert!(snapshot.problems.is_empty(), "{:?}", snapshot.problems);
}

#[test]
fn snapshot_reads_a_running_codex_review_on_a_stacked_pull() {
    let mut case = Case::recorded(3033);
    case.rules = Ok("[[]]".to_owned());
    let snapshot = snapshot(&case);
    assert_eq!(snapshot.state, PullState::Open);
    assert_eq!(snapshot.base_ref, "feat/issue-2899");
    assert_eq!(snapshot.mergeable, Some(true));
    assert_eq!(snapshot.codex.expect("codex").security_status, "running");
    assert!(!snapshot.queue.has_queue && snapshot.problems.is_empty());
}

#[test]
fn snapshot_a_403_on_the_rules_means_no_required_checks() {
    let mut case = Case::recorded(2963);
    case.rules = Err("gh: Forbidden (HTTP 403)".to_owned());
    let snapshot = snapshot(&case);
    assert!(snapshot.required_checks.is_empty() && snapshot.problems.is_empty());
}

#[test]
fn snapshot_check_runs_over_one_page_are_a_problem() {
    let mut case = Case::recorded(2963);
    case.checks = r#"{"total_count":101,"check_runs":[{"name":"a","status":"queued"}]}"#.to_owned();
    let snapshot = snapshot(&case);
    assert_eq!(snapshot.checks.len(), 1);
    assert!(!snapshot.checks[0].completed && snapshot.problems[0].contains("101 check runs"));
}

#[test]
fn snapshot_maps_commit_statuses_to_checks() {
    let mut case = Case::recorded(2963);
    case.status = r#"{"total_count":3,"statuses":[{"context":"a","state":"success"},{"context":"b","state":"error"},{"context":"c","state":"pending"}]}"#.to_owned();
    let found: Vec<_> = snapshot(&case)
        .checks
        .iter()
        .skip(16)
        .map(|c| (c.completed, c.conclusion.clone()))
        .collect();
    let want = [
        (true, Some("success")),
        (true, Some("failure")),
        (false, None),
    ];
    assert_eq!(
        found,
        want.map(|(done, text)| (done, text.map(str::to_owned)))
    );
}

#[test]
fn snapshot_pages_the_threads_by_cursor() {
    let mut case = Case::recorded(2963);
    case.graphs = vec![page(2, Some("C1"), 1), page(1, None, 1)];
    let gh = case.gh();
    let snapshot = read(&gh, &case.link()).expect("the pull is read");
    assert_eq!(snapshot.threads.len(), 3);
    let calls = gh.calls();
    let inputs: Vec<_> = calls
        .iter()
        .filter_map(|(_, input)| input.as_deref())
        .collect();
    assert!(inputs[0].contains(r#""cursor":null"#) && inputs[1].contains(r#""cursor":"C1""#));
}

#[test]
fn snapshot_stops_after_ten_pages_and_says_so() {
    let mut case = Case::recorded(2963);
    case.graphs = vec![page(1, Some("C"), 1); 11];
    let snapshot = snapshot(&case);
    assert_eq!(snapshot.threads.len(), 10);
    assert!(snapshot.problems[0].contains("10 pages"));
}

#[test]
fn snapshot_a_thread_of_over_100_comments_is_a_problem() {
    let mut case = Case::recorded(2963);
    case.graphs = vec![page(1, None, 101)];
    assert!(snapshot(&case).problems[0].contains("101 comments"));
}

#[test]
fn snapshot_a_failed_read_is_a_problem_and_a_failed_pull_is_an_error() {
    let mut case = Case::recorded(2963);
    case.graphs.clear();
    let problem = snapshot(&case).problems.remove(0);
    assert!(problem.starts_with("The review threads and the merge queue could not be read"));
    assert!(read(&FakeGh::new(), &case.link()).is_err());
}

#[test]
fn snapshot_a_codex_comment_that_does_not_parse_is_a_problem() {
    let mut case = Case::recorded(2963);
    case.comments = case.comments.replace(r#"{\"code_review\""#, r#"{\"nope\""#);
    let snapshot = snapshot(&case);
    assert!(snapshot.codex.is_none() && snapshot.problems[0].contains("Codex"));
}

#[test]
fn snapshot_verdict_is_the_last_non_blank_line() {
    let sha = "0123456789abcdef0123456789abcdef01234567";
    let line = |word: &str| format!("Findings.\n\nVERDICT: {word} head={sha}\r\n\n  \n");
    assert!(verdict(&line("PASS"), "t").expect("pass").pass);
    assert!(!verdict(&line("FAIL"), "t").expect("fail").pass);
    assert!(verdict(&format!("VERDICT: PASS head={sha}\nmore"), "t").is_none());
    assert!(verdict("VERDICT: PASS head=abc", "t").is_none());
    assert!(verdict(&line("MAYBE"), "t").is_none());
}

#[test]
fn snapshot_reads_the_place_in_the_merge_queue() {
    let snapshot = snapshot(&Case::recorded(3084));
    assert_eq!(snapshot.state, PullState::Open);
    assert_eq!(
        (snapshot.mergeable, snapshot.mergeable_state.as_str()),
        (Some(true), "clean")
    );
    let entry = snapshot.queue.entry.expect("an entry");
    assert_eq!(
        (entry.state.as_str(), entry.position),
        ("AWAITING_CHECKS", Some(1))
    );
    assert!(snapshot.queue.has_queue && snapshot.problems.is_empty());
}
