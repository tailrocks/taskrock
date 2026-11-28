//! Tests of who gives a verdict: only a review in the format of the manager.

use super::case_tests::Case;
use super::read;
use super::rest::verdict;

const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

/// A review (`submitted_at`) with this body, as GitHub lists it.
fn review(body: &str) -> String {
    let review = serde_json::json!({"user": {"login": "r"}, "submitted_at": "t", "body": body});
    format!("[[{review}]]")
}

fn found(reviews: &str, comments: &str) -> usize {
    let mut case = Case::recorded(2963);
    case.reviews = reviews.to_owned();
    case.comments = comments.to_owned();
    let snapshot = read(&case.gh(), &case.link()).expect("the pull is read");
    snapshot.verdicts.len()
}

#[test]
fn snapshot_a_comment_with_a_pass_line_gives_no_verdict() {
    let body = format!("Reviewer: r\nVERDICT: PASS head={SHA}");
    let comment = serde_json::json!({"user": {"login": "r"}, "created_at": "t", "body": body});
    assert_eq!(found("[[]]", &format!("[[{comment}]]")), 0);
    // The same body as a review does count, so only the source differs.
    assert_eq!(found(&review(&body), "[[]]"), 1);
}

#[test]
fn snapshot_a_review_with_no_reviewer_line_gives_no_verdict() {
    let pass = format!("VERDICT: PASS head={SHA}");
    assert_eq!(found(&review(&pass), "[[]]"), 0);
    assert_eq!(
        found(&review(&format!("Not Reviewer: r\n{pass}")), "[[]]"),
        0
    );
    assert_eq!(found(&review(&format!(" Reviewer: r\n{pass}")), "[[]]"), 0);
}

#[test]
fn snapshot_a_review_with_both_lines_gives_a_verdict() {
    let body = format!("Reviewer: `agent` on a model.\nFindings.\n\nVERDICT: FAIL head={SHA}\n");
    assert_eq!(found(&review(&body), "[[]]"), 1);
    // The verdict must stay the last non-blank line.
    assert_eq!(found(&review(&format!("{body}more")), "[[]]"), 0);
}

#[test]
fn snapshot_verdict_is_the_last_non_blank_line() {
    let line = |word: &str| format!("Reviewer: r\n\nVERDICT: {word} head={SHA}\r\n\n  \n");
    assert!(verdict(&line("PASS"), "t").expect("pass").pass);
    assert!(!verdict(&line("FAIL"), "t").expect("fail").pass);
    let more = format!("Reviewer: r\nVERDICT: PASS head={SHA}\nmore");
    assert!(verdict(&more, "t").is_none());
    assert!(verdict("Reviewer: r\nVERDICT: PASS head=abc", "t").is_none());
    assert!(verdict(&line("MAYBE"), "t").is_none());
}

#[test]
fn snapshot_the_head_of_a_verdict_is_lowercase() {
    let body = format!("Reviewer: r\nVERDICT: PASS head={}", SHA.to_uppercase());
    assert_eq!(verdict(&body, "t").expect("a verdict").head, SHA);
}
