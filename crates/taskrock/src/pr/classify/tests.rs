//! Tests of the thread classes.

use super::{Class, classify};
use crate::pr::snapshot::ThreadFact;

fn thread(author: &str, is_bot: bool, body: &str) -> ThreadFact {
    ThreadFact {
        id: "T".into(),
        resolved: false,
        url: "u".into(),
        first_comment_id: 1,
        first_author: author.into(),
        first_is_bot: is_bot,
        first_body: body.into(),
        bodies: vec![body.into()],
    }
}

const P0: &str = "![P0 Badge](https://img.shields.io/badge/P0-red)";
const P2: &str = "![P2 Badge](https://img.shields.io/badge/P2-yellow)";
const P3: &str = "![P3 Badge](https://img.shields.io/badge/P3-green)";

#[test]
fn classify_a_bot_p0_to_p2_thread_is_blocking() {
    for body in [P0, P2] {
        assert_eq!(classify(&thread("x", true, body)), Class::Blocking);
    }
}

#[test]
fn classify_a_bot_thread_without_a_severity_is_blocking() {
    assert_eq!(classify(&thread("x", true, "Fix this.")), Class::Blocking);
}

#[test]
fn classify_a_bot_p3_thread_is_deferrable() {
    assert_eq!(classify(&thread("x", true, P3)), Class::Deferrable);
}

#[test]
fn classify_a_login_ending_in_bot_is_a_bot_in_any_case() {
    for login in ["chatgpt-codex-connector[bot]", "Codex[BOT]"] {
        assert_eq!(classify(&thread(login, false, P3)), Class::Deferrable);
    }
}

#[test]
fn classify_a_person_is_human_whatever_the_severity() {
    assert_eq!(classify(&thread("donbeave", false, P0)), Class::Human);
    assert_eq!(classify(&thread("donbeave", false, "ok")), Class::Human);
}

#[test]
fn classify_names_are_stable() {
    let names = [Class::Blocking, Class::Deferrable, Class::Human].map(Class::as_str);
    assert_eq!(names, ["blocking", "deferrable", "human"]);
}
