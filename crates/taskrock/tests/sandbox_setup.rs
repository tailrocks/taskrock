//! Run `setup` twice on the sandbox project: the second run changes nothing.

use std::path::PathBuf;

use taskrock::gh::RealGh;
use taskrock::setup::{Options, run};

fn options() -> Options {
    Options {
        owner: "tailrocks".to_owned(),
        file: Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/config-sandbox.toml")),
        dry_run: false,
        yes: false,
    }
}

#[test]
#[ignore = "talks to GitHub"]
fn sandbox_setup_second_run_changes_nothing() {
    let gh = RealGh::default();
    let first = run(&gh, &options()).expect("first setup");
    let second = run(&gh, &options()).expect("second setup");
    assert!(
        first
            .lines()
            .last()
            .is_some_and(|line| line.starts_with("DONE ")),
        "first run ends with DONE: {first}"
    );
    assert!(
        second.lines().all(|line| !line.starts_with("create")
            && !line.starts_with("update")
            && !line.starts_with("delete")),
        "second run changes nothing: {second}"
    );
    assert_eq!(
        second.lines().last(),
        Some("DONE 0 changes"),
        "second run ends with DONE 0 changes: {second}"
    );
}
