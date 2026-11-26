//! Tests of the refusals of `claim` for an issue in Triage.

use crate::error::Error;

use super::claim;
use super::tests::{TASK, URL, full, has};

#[test]
fn a_triage_issue_names_taskrock_triage() {
    let bad = TASK
        .replace("\"Task\"", "\"Bug\"")
        .replace("Todo", "Triage");
    let gh = full(&bad, None);
    let error = claim(&gh, URL).expect_err("claim is refused");
    assert!(matches!(
        &error,
        Error::Refused(got)
            if got == "the issue is in Triage. Triage it first with the skill taskrock-triage"
    ));
    assert!(!has(&gh, "POST"), "no branch is created");
}
