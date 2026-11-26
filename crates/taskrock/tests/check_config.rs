//! `taskrock check` with a config file.

use std::process::Command;

fn check(file: &str) -> std::io::Result<(String, String, Option<i32>)> {
    let output = Command::new(env!("CARGO_BIN_EXE_taskrock"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["check", file])
        .output()?;
    Ok((
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code(),
    ))
}

#[test]
fn a_good_config_is_ok() -> std::io::Result<()> {
    let (stdout, _, code) = check("fixtures/config-good.toml")?;
    assert_eq!(stdout.trim(), "OK config, 2 projects");
    assert_eq!(code, Some(0));
    Ok(())
}

#[test]
fn a_bad_config_prints_each_problem_and_exits_with_one() -> std::io::Result<()> {
    let (stdout, stderr, code) = check("fixtures/config-bad.toml")?;
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3, "{stdout}");
    assert!(
        lines
            .iter()
            .all(|line| line.starts_with("fixtures/config-bad.toml: "))
    );
    assert_eq!(stderr.trim(), "error: 3 problems in the draft");
    assert_eq!(code, Some(1));
    Ok(())
}

#[test]
fn a_draft_folder_is_still_checked_as_a_draft() -> std::io::Result<()> {
    let (stdout, _, code) = check("fixtures/draft-good")?;
    assert_eq!(stdout.trim(), "OK 3 issues");
    assert_eq!(code, Some(0));
    Ok(())
}
