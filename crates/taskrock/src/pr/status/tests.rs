//! Tests of the exit code and of the output of `taskrock pr status`.

use serde_json::Value;

use super::evaluate::State;
use super::{outcome, verdict};
use crate::pr::snapshot::case_tests::Case;
use crate::pr::{EXIT_BLOCKED, EXIT_DONE, EXIT_WAITING};

/// The output and the exit code of `status` on a recorded pull request.
fn recorded(number: u64, json: bool) -> (State, String, u8) {
    let case = Case::recorded(number);
    let link = case.link();
    let gh = case.gh().fail("taskrock.toml", "gh: Not Found (HTTP 404)");
    let (snapshot, judged) = verdict(&gh, &link).expect("the pull is judged");
    let (out, code) = outcome(&link, &snapshot, &judged, json);
    (judged.state, out, code)
}

#[test]
fn status_run_a_waiting_pull_exits_with_waiting() {
    let (state, _, code) = recorded(3084, false);
    assert_eq!(state, State::Waiting);
    assert_eq!(code, EXIT_WAITING);
}

#[test]
fn status_run_a_blocked_pull_exits_with_blocked() {
    let (state, _, code) = recorded(2963, false);
    assert_eq!(state, State::Blocked);
    assert_eq!(code, EXIT_BLOCKED);
}

#[test]
fn status_run_a_merged_pull_exits_with_done() {
    let (state, _, code) = recorded(2932, false);
    assert_eq!(state, State::Merged);
    assert_eq!(code, EXIT_DONE);
}

#[test]
fn status_run_a_ready_pull_exits_with_done() {
    let case = Case::recorded(2932);
    let link = case.link();
    let gh = case.gh().fail("taskrock.toml", "gh: Not Found (HTTP 404)");
    let (snapshot, mut judged) = verdict(&gh, &link).expect("the pull is judged");
    judged.state = State::Ready;
    assert_eq!(outcome(&link, &snapshot, &judged, false).1, EXIT_DONE);
}

#[test]
fn status_run_the_json_flag_switches_the_output() {
    let (_, text, _) = recorded(3084, false);
    let (_, json, _) = recorded(3084, true);
    assert!(
        text.starts_with("tailrocks/velnor-new#3084: WAITING"),
        "{text}"
    );
    let value: Value = serde_json::from_str(&json).expect("JSON");
    assert_eq!(value["state"], "WAITING");
}
