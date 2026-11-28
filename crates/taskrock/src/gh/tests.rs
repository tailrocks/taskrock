//! Tests of the scripted `Gh`.

use crate::error::Error;
use crate::gh::FakeGh;
use crate::gh::Gh as _;

#[test]
fn replies_are_used_once_in_order() {
    let gh = FakeGh::new().ok("one", "first").ok("one", "second");
    let first = gh.run(&["get", "one"], None).expect("first reply");
    let second = gh.run(&["get", "one"], None).expect("second reply");
    assert_eq!(first, "first");
    assert_eq!(second, "second");
}

#[test]
fn call_without_reply_fails() {
    let gh = FakeGh::new();
    let error = gh.run(&["api", "missing"], None).expect_err("no reply");
    match error {
        Error::Gh { args, stderr, .. } => {
            assert_eq!(args, "api missing");
            assert_eq!(stderr, "no fake reply for: api missing");
        }
        other => panic!("expected Error::Gh, got {other}"),
    }
}

#[test]
fn calls_returns_arguments_and_input() {
    let gh = FakeGh::new().ok("api", "{}");
    gh.run(&["api", "path"], Some("body")).expect("reply");
    assert_eq!(
        gh.calls(),
        [("api path".to_owned(), Some("body".to_owned()))]
    );
}
