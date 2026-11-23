//! The binary runs and prints its version.

use std::process::Command;

#[test]
fn prints_name_and_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_taskrock"))
        .output()
        .expect("the taskrock binary starts");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert_eq!(stdout.trim(), "taskrock 0.1.0");
}
