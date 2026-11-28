//! Tests of the facts that a recorded pull request does not show.

use super::case_tests::{Case, page, queued};
use super::read;
use super::rest::verdict;
use crate::pr::status;
use serde_json::json;

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

/// One review or comment with a verdict `word`, at `at` under `time_key`.
fn note(time_key: &str, at: &str, word: &str) -> String {
    format!(
        r#"{{"user":{{"login":"r"}},"{time_key}":"{at}","body":"Findings.\n\nVERDICT: {word} head={SHA}"}}"#
    )
}

#[test]
fn snapshot_verdicts_of_comments_and_reviews_are_ordered_by_time() {
    let mut case = Case::recorded(2963);
    case.reviews = format!(
        "[[{}]]",
        note("submitted_at", "2026-02-01T00:00:00Z", "PASS")
    );
    let late = note("created_at", "2026-03-01T00:00:00Z", "FAIL");
    let early = note("created_at", "2026-01-01T00:00:00Z", "PASS");
    case.comments = format!("[[{late},{early}]]");
    let found = read(&case.gh(), &case.link())
        .expect("the pull is read")
        .verdicts;
    let found: Vec<_> = found.iter().map(|v| (v.at.as_str(), v.pass)).collect();
    assert_eq!(
        found,
        [
            ("2026-01-01T00:00:00Z", true),
            ("2026-02-01T00:00:00Z", true),
            ("2026-03-01T00:00:00Z", false),
        ]
    );
}

#[test]
fn snapshot_the_merge_queue_rule_alone_means_a_queue() {
    let mut case = Case::recorded(2963);
    case.graphs = vec![page(1, None, 1)];
    assert!(
        read(&case.gh(), &case.link())
            .expect("read")
            .queue
            .has_queue
    );
    case.rules = Ok("[[]]".to_owned());
    assert!(
        !read(&case.gh(), &case.link())
            .expect("read")
            .queue
            .has_queue
    );
}

#[test]
fn snapshot_an_error_on_a_later_page_keeps_the_earlier_pages() {
    let mut case = Case::recorded(2963);
    case.rules = Ok("[[]]".to_owned());
    case.graphs = vec![queued(&page(3, Some("C1"), 101))];
    let gh = case.gh().fail("graphql", "gh: Bad Gateway (HTTP 502)");
    let snapshot = read(&gh, &case.link()).expect("the pull is read");
    assert_eq!(snapshot.threads.len(), 3);
    assert!(snapshot.queue.has_queue);
    let entry = snapshot.queue.entry.as_ref();
    assert_eq!(
        entry.map(|e| (e.state.as_str(), e.position)),
        Some(("QUEUED", Some(2)))
    );
    let problems = &snapshot.problems;
    assert_eq!(problems.len(), 4, "{problems:?}");
    assert_eq!(
        problems
            .iter()
            .filter(|p| p.contains("101 comments"))
            .count(),
        3
    );
    let last = &problems[3];
    assert!(
        last.contains("later pages") && last.contains("only the 3 threads"),
        "{last}"
    );
}

#[test]
fn snapshot_the_head_of_a_verdict_is_lowercase() {
    let found = verdict(&format!("VERDICT: PASS head={}", SHA.to_uppercase()), "t");
    assert_eq!(found.expect("a verdict").head, SHA);
}

/// The names and conclusions of the checks and the finding codes that `status` gives, when the
/// head has the check runs `runs`: `(id, name, conclusion)`, in the order GitHub lists them.
fn judged_runs(runs: &[(u64, &str, &str)]) -> (Vec<(String, String)>, Vec<&'static str>) {
    let mut case = Case::recorded(2963);
    let runs: Vec<_> = runs
        .iter()
        .map(|(id, name, conclusion)| json!({"id": id, "name": name, "status": "completed", "conclusion": conclusion}))
        .collect();
    case.checks = json!({"total_count": runs.len(), "check_runs": runs}).to_string();
    case.status = r#"{"total_count":0,"statuses":[]}"#.to_owned();
    let gh = case.gh().fail("taskrock.toml", "gh: Not Found (HTTP 404)");
    let (snapshot, judged) = status::verdict(&gh, &case.link()).expect("the pull is judged");
    let checks = snapshot.checks.into_iter();
    let checks = checks.map(|c| (c.name, c.conclusion.unwrap_or_default()));
    let codes = judged.findings.iter().map(|f| f.code.as_str());
    (checks.collect(), codes.collect())
}

#[test]
fn snapshot_keeps_only_the_newest_run_of_a_check_name() {
    let pair = |old: &'static str, new: &'static str, newest_first: bool| {
        let mut runs = vec![
            (10, "Required", old),
            (20, "Other", "success"),
            (30, "Required", new),
        ];
        if newest_first {
            runs.reverse();
        }
        judged_runs(&runs)
    };
    for newest_first in [false, true] {
        let (checks, codes) = pair("failure", "success", newest_first);
        assert_eq!(checks.len(), 2, "{checks:?}");
        assert!(checks.contains(&("Required".to_owned(), "success".to_owned())));
        assert!(!codes.contains(&"check_failed"), "{codes:?}");
        let (checks, codes) = pair("success", "failure", newest_first);
        assert!(checks.contains(&("Required".to_owned(), "failure".to_owned())));
        assert!(codes.contains(&"check_failed"), "{codes:?}");
    }
    // A name keeps the place of its first appearance.
    let (checks, _) = judged_runs(&[
        (10, "Required", "failure"),
        (20, "Other", "success"),
        (30, "Required", "success"),
    ]);
    assert_eq!(checks[0].0, "Required");
}
