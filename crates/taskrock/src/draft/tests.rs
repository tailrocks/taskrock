//! Tests for reading a draft folder.

use std::path::Path;

use super::{
    DraftIssue,
    parse::{parse_issue, parse_plan, read},
};
use crate::error::Error;

const PLAN: &str = "id = \"holla-csv\"\nrepo = \"tailrocks/holla\"\nproject = \"https://github.com/orgs/tailrocks/projects/7\"\n";
const ISSUE: &str =
    "+++\nkey = \"T01\"\ntitle = \"Render rows\"\ntype = \"Task\"\n+++\n\n## Outcome\n\nDone.\n";

fn fails_with<T>(result: Result<T, Error>) -> (String, String) {
    let Err(Error::Draft { file, message }) = result else {
        panic!("expected Error::Draft");
    };
    (file, message)
}

#[test]
fn read_good_returns_four_issues_in_file_order() {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/draft-good");
    let draft = read(&folder).expect("draft-good reads");
    let plan = &draft.plan;
    assert_eq!(plan.id, "holla-csv");
    assert_eq!(plan.repo, "tailrocks/holla");
    assert_eq!(plan.project.owner, "tailrocks");
    assert_eq!(plan.project.number, 7);
    assert_eq!(plan.milestones.len(), 1);
    let keys = draft.issues.iter().map(|issue| issue.key.as_str());
    assert!(keys.eq(["E1", "I1", "T01", "T02"]));
    let files = draft.issues.iter().map(|issue| issue.file.as_str());
    assert!(files.eq(["E1.md", "I1.md", "T01.md", "T02.md"]));
    assert_eq!(draft.issues[0].parent.as_deref(), Some("I1"));
    assert_eq!(draft.issues[1].parent, None);
    assert_eq!(draft.issues[2].parent.as_deref(), Some("E1"));
    assert_eq!(draft.issues[3].blocked_by, ["T01"]);
    for issue in &draft.issues[2..] {
        assert!(
            issue.routing.missing().is_empty(),
            "{} misses routing",
            issue.key
        );
    }
    assert_eq!(
        draft.issues[2].routing.get("Agent"),
        Some("tailrocks-systems-engineer")
    );
    assert!(draft.issues[3].body.contains("## Out of scope"));
}

#[test]
fn missing_plan_reports_the_file() {
    let dir = std::env::temp_dir().join("taskrock-test-without-plan");
    std::fs::create_dir_all(&dir).expect("the temp dir exists");
    let expected = ("plan.toml".to_owned(), "plan.toml is missing".to_owned());
    assert_eq!(fails_with(read(&dir)), expected);
}

#[test]
fn unreadable_folder_reports_the_io_error() {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/draft-absent");
    let (file, message) = fails_with(read(&folder));
    assert_eq!(file, folder.display().to_string());
    assert!(message.contains("No such file"), "io error, got: {message}");
}

#[test]
fn issue_must_start_with_delimiter() {
    let result = parse_issue("T01.md", "key = \"T01\"\n", "tailrocks/holla");
    assert_eq!(fails_with(result).1, "the file must start with a line +++");
}

#[test]
fn issue_without_second_delimiter_fails() {
    let result = parse_issue("T01.md", "+++\nkey = \"T01\"\n", "tailrocks/holla");
    assert_eq!(fails_with(result).1, "the second +++ line is missing");
}

#[test]
fn broken_toml_reports_the_toml_error() {
    let result = parse_issue("T01.md", "+++\nkey = \n+++\nBody.\n", "tailrocks/holla");
    let (file, message) = fails_with(result);
    assert_eq!(file, "T01.md");
    assert!(message.contains("line 1"), "toml error, got: {message}");
}

#[test]
fn unknown_key_reports_the_toml_error() {
    let text = ISSUE.replace("type = \"Task\"", "type = \"Task\"\nbogus = \"x\"");
    let Err(error) = parse_issue("T01.md", &text, "tailrocks/holla") else {
        panic!("the key is unknown")
    };
    let display = error.to_string();
    let (file, message) = fails_with::<DraftIssue>(Err(error));
    assert!(message.contains("bogus"), "toml error, got: {message}");
    assert_eq!(display, format!("{file}: {message}"));
}

#[test]
fn bad_plan_id_reports_the_name() {
    let result = parse_plan(&PLAN.replace("holla-csv", "no good"));
    assert_eq!(
        fails_with(result).1,
        "only letters, digits and - are allowed in id"
    );
}

#[test]
fn bad_issue_key_reports_the_name() {
    let text = ISSUE.replace("key = \"T01\"", "key = \"T 01\"");
    let result = parse_issue("T01.md", &text, "tailrocks/holla");
    assert_eq!(
        fails_with(result).1,
        "only letters, digits and - are allowed in key"
    );
}

#[test]
fn bad_repo_reports_the_shape() {
    let result = parse_plan(&PLAN.replace("tailrocks/holla", "holla"));
    assert_eq!(fails_with(result).1, "repo must be owner/name");
    let text = ISSUE.replace("type = \"Task\"", "type = \"Task\"\nrepo = \"holla\"");
    let result = parse_issue("T01.md", &text, "tailrocks/holla");
    assert_eq!(fails_with(result).1, "repo must be owner/name");
}

#[test]
fn body_drops_leading_blank_lines_and_trailing_space() {
    let text = "+++\nkey = \"T01\"\ntitle = \"T\"\ntype = \"Task\"\n+++\n\n\nBody.\n   \n";
    let issue = parse_issue("T01.md", text, "tailrocks/holla").expect("the issue parses");
    assert_eq!(issue.body, "Body.");
}

#[test]
fn issue_repo_defaults_to_the_plan_repo() {
    let fallback = parse_issue("T01.md", ISSUE, "tailrocks/holla").expect("the issue parses");
    assert_eq!(fallback.repo, "tailrocks/holla");
    let text = ISSUE.replace(
        "type = \"Task\"",
        "type = \"Task\"\nrepo = \"tailrocks/other\"",
    );
    let own = parse_issue("T01.md", &text, "tailrocks/holla").expect("the issue parses");
    assert_eq!(own.repo, "tailrocks/other");
}
