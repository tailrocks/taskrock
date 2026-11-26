//! Tests for `taskrock template`.

use std::path::Path;

use super::run;
use crate::error::Error;
use crate::gh::FakeGh;

const GOOD: &str = include_str!("../../fixtures/config-good.toml");
const FILE: &str = "contents/taskrock.toml";

fn default_template(kind: &str) -> String {
    run(&FakeGh::new(), kind, None, None).expect("a default type has a template")
}

#[test]
fn bug_has_its_four_file_sections_and_no_trailing_line() {
    let text = default_template("Bug");
    assert_eq!(
        text,
        "## What happens\n\n\n## Expected\n\n\n## How to reproduce\n\n\n## Environment"
    );
}

#[test]
fn a_checklist_section_has_an_empty_box() {
    let text = default_template("Task");
    assert_eq!(
        text,
        "## Outcome\n\n\n## Decisions already made\n\n\n## Steps\n\n\n\
         ## Done when\n\n- [ ] `<command>` <result>\n\n## Out of scope"
    );
}

#[test]
fn every_default_type_prints_its_file_sections() {
    let counts = [
        ("Initiative", 5),
        ("Epic", 5),
        ("Task", 5),
        ("Bug", 4),
        ("Feature", 3),
        ("owner", 5),
    ];
    for (kind, count) in counts {
        let text = default_template(kind);
        let headings = text.lines().filter(|line| line.starts_with("## ")).count();
        assert_eq!(headings, count, "{kind}");
        assert!(!text.ends_with('\n'), "{kind} has a trailing line");
    }
}

#[test]
fn owner_prints_the_owner_task_format() {
    let text = default_template("owner");
    assert_eq!(text.lines().next(), Some("## Outcome"));
    assert!(text.contains("## Recommended answer"));
    assert!(text.ends_with("- [ ] `<command>` <result>"));
}

#[test]
fn an_unknown_type_lists_the_types() {
    let Err(Error::Config(message)) = run(&FakeGh::new(), "Story", None, None) else {
        panic!("an unknown type is an error");
    };
    assert_eq!(
        message,
        "unknown type Story. Types: Initiative, Epic, Task, Bug, Feature, owner"
    );
}

#[test]
fn the_owner_flag_reads_the_config_of_the_org() {
    let custom = GOOD.replace(
        r#"file      = ["Problem", "Proposed behavior", "Workaround today"]"#,
        r#"file      = ["Problem", "Done when"]"#,
    );
    let gh = FakeGh::new().ok(FILE, &custom);
    let text = run(&gh, "Feature", Some("acme"), None).expect("the org config loads");
    assert_eq!(
        text,
        "## Problem\n\n\n## Done when\n\n- [ ] `<command>` <result>"
    );
    assert!(
        gh.calls()[0]
            .0
            .contains("repos/acme/.github/contents/taskrock.toml")
    );
}

#[test]
fn the_owner_flag_falls_back_to_the_default_on_http_404() {
    let gh = FakeGh::new().fail(FILE, "gh: Not Found (HTTP 404)");
    let text = run(&gh, "Bug", Some("acme"), None).expect("a missing file is the default");
    assert_eq!(text, default_template("Bug"));
}

#[test]
fn the_config_flag_reads_a_local_file_and_skips_github() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/config-good.toml");
    let gh = FakeGh::new();
    let text = run(&gh, "Bug", Some("acme"), Some(&path)).expect("the file loads");
    assert_eq!(text, default_template("Bug"));
    assert_eq!(gh.calls().len(), 0);
}

#[test]
fn a_missing_config_file_is_an_error() {
    let result = run(
        &FakeGh::new(),
        "Bug",
        None,
        Some(Path::new("/no/such/file.toml")),
    );
    assert!(matches!(result, Err(Error::Config(_))));
}
