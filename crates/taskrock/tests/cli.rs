//! The command line of taskrock.

use std::process::Command;

#[test]
fn version_prints_name_and_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_taskrock"))
        .arg("--version")
        .output()
        .expect("the taskrock binary starts");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert_eq!(stdout.trim(), "taskrock 0.5.1");
}

#[test]
fn help_lists_every_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_taskrock"))
        .arg("--help")
        .output()
        .expect("the taskrock binary starts");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    for command in [
        "check", "apply", "ready", "claim", "release", "setup", "template", "edit", "pr",
    ] {
        assert!(stdout.contains(command), "help lists {command}");
    }
}

#[test]
fn no_argument_exits_with_wrong_arguments() {
    let output = Command::new(env!("CARGO_BIN_EXE_taskrock"))
        .output()
        .expect("the taskrock binary starts");
    assert_eq!(output.status.code(), Some(2));
}

fn threads_exit_code(args: &[&str]) -> std::io::Result<Option<i32>> {
    let output = Command::new(env!("CARGO_BIN_EXE_taskrock"))
        .args(["pr", "threads", "tailrocks/taskrock#1"])
        .args(args)
        .output()?;
    Ok(output.status.code())
}

#[test]
fn threads_defer_with_a_reason_exits_with_wrong_arguments() {
    assert_eq!(
        threads_exit_code(&["--defer", "--reason", "y"]).expect("the taskrock binary starts"),
        Some(2)
    );
}

#[test]
fn threads_resolve_needs_a_reason_and_a_reason_needs_resolve() {
    assert_eq!(
        threads_exit_code(&["--resolve", "X"]).expect("the taskrock binary starts"),
        Some(2)
    );
    assert_eq!(
        threads_exit_code(&["--reason", "y"]).expect("the taskrock binary starts"),
        Some(2)
    );
}

/// A pull request that is merged, so that no command writes to it.
const MERGED_PULL: &str = "https://github.com/tailrocks/taskrock/pull/153";

fn run_binary(args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new(env!("CARGO_BIN_EXE_taskrock"))
        .args(args)
        .output()
}

#[test]
#[ignore = "talks to GitHub"]
fn pr_status_json_prints_the_verdict_as_json() {
    let output =
        run_binary(&["pr", "status", MERGED_PULL, "--json"]).expect("the taskrock binary starts");
    assert_eq!(output.status.code(), Some(0));
    let verdict: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is JSON");
    assert_eq!(verdict["state"], "MERGED");
}

#[test]
#[ignore = "talks to GitHub"]
fn pr_threads_json_prints_the_report_as_json() {
    let output = run_binary(&["pr", "threads", MERGED_PULL, "--defer", "--json"])
        .expect("the taskrock binary starts");
    assert_eq!(output.status.code(), Some(0));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("stdout is JSON");
    assert_eq!(report["resolved"], serde_json::json!([]));
    assert_eq!(report["failed"], serde_json::Value::Null);
}

#[test]
#[ignore = "talks to GitHub"]
fn pr_threads_resolve_json_refuses_and_writes_nothing() {
    let output = run_binary(&[
        "pr",
        "threads",
        MERGED_PULL,
        "--resolve",
        "X",
        "--reason",
        "test",
        "--json",
    ])
    .expect("the taskrock binary starts");
    assert_eq!(output.status.code(), Some(3));
    assert_eq!(output.stdout, b"");
}

#[test]
#[ignore = "talks to GitHub"]
fn pr_merge_json_prints_the_report_as_json() {
    let output =
        run_binary(&["pr", "merge", MERGED_PULL, "--json"]).expect("the taskrock binary starts");
    assert_eq!(output.status.code(), Some(0));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("stdout is JSON");
    assert_eq!(report["outcome"], "already_merged");
}
