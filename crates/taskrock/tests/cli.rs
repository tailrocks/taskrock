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
    assert_eq!(stdout.trim(), "taskrock 0.3.0");
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
        "check", "apply", "ready", "claim", "release", "setup", "template", "edit",
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
