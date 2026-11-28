//! Tests of the facts that a recorded pull request does not show.

use super::case_tests::{Case, page, queued};
use super::read;
use super::rest::verdict;

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
