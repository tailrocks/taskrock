//! Every skill stays in step with the commands of the tool.

use std::path::PathBuf;
use std::process::Command;

fn skills_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../skills")
}

fn skill_files() -> std::io::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(skills_dir())? {
        let path = entry?.path();
        if path.is_dir() {
            files.push(path.join("SKILL.md"));
        }
    }
    files.sort();
    Ok(files)
}

fn command_of(line: &str) -> Option<&str> {
    line.split_whitespace().nth(1)
}

#[test]
fn front_matter_names_the_skill() {
    let files = skill_files().expect("the skills folder reads");
    assert!(!files.is_empty(), "at least one skill exists");
    for file in &files {
        let folder = file
            .parent()
            .expect("a skill file has a parent")
            .file_name()
            .expect("a skill folder has a name")
            .to_string_lossy()
            .into_owned();
        let text = std::fs::read_to_string(file).expect("a skill file reads");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "---", "{} starts with ---", file.display());
        assert_eq!(
            lines[1],
            format!("name: {folder}"),
            "{} names {folder}",
            file.display()
        );
        assert!(
            lines[2].starts_with("description:"),
            "{} has a description",
            file.display()
        );
        assert_eq!(
            lines[3],
            "---",
            "{} closes the front matter",
            file.display()
        );
    }
}

#[test]
fn skills_are_short() {
    let files = skill_files().expect("the skills folder reads");
    assert!(!files.is_empty(), "at least one skill exists");
    for file in &files {
        let text = std::fs::read_to_string(file).expect("a skill file reads");
        assert!(
            text.lines().count() <= 500,
            "{} has at most 500 lines",
            file.display()
        );
    }
}

#[test]
fn skill_commands_exist() {
    let files = skill_files().expect("the skills folder reads");
    assert!(!files.is_empty(), "at least one skill exists");
    for file in &files {
        let text = std::fs::read_to_string(file).expect("a skill file reads");
        let mut in_fence = false;
        for line in text.lines() {
            if line.starts_with("```") {
                in_fence = !in_fence;
                continue;
            }
            if in_fence && line.starts_with("taskrock ") {
                let command = command_of(line).expect("a taskrock line names a command");
                let output = Command::new(env!("CARGO_BIN_EXE_taskrock"))
                    .arg(command)
                    .arg("--help")
                    .output()
                    .expect("the taskrock binary starts");
                assert!(
                    output.status.success(),
                    "{command} from {} has --help",
                    file.display()
                );
            }
        }
    }
}
