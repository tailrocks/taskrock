//! Tests for the coverage notes.

use std::fmt::Write as _;
use std::path::Path;

use super::{notes, run};
use crate::config;
use crate::draft::parse::{parse_issue, parse_plan, read};
use crate::draft::{Draft, DraftIssue};

const PLAN: &str =
    "id = \"p\"\nrepo = \"o/n\"\nproject = \"https://github.com/orgs/o/projects/7\"\n";
const ROUTED: &str = "agent = \"a\"\nruntime = \"b\"\nmodel = \"c\"\neffort = \"d\"\nreview_agent = \"e\"\nreview_runtime = \"f\"\nreview_model = \"g\"\nreview_effort = \"h\"\n";

fn issue(key: &str, kind: &str, extra: &str, body: &str) -> DraftIssue {
    let text =
        format!("+++\nkey = \"{key}\"\ntitle = \"{key}\"\ntype = \"{kind}\"\n{extra}+++\n{body}");
    parse_issue(&format!("{key}.md"), &text, "o/n").expect("the test issue parses")
}

fn task(key: &str, extra: &str, body: &str) -> DraftIssue {
    issue(key, "Task", &format!("{ROUTED}{extra}"), body)
}

fn lines(issues: Vec<DraftIssue>) -> Vec<String> {
    let plan = parse_plan(PLAN).expect("the test plan parses");
    notes(&Draft { plan, issues })
        .into_iter()
        .map(|note| format!("{}: {}", note.file, note.message))
        .collect()
}

fn numbered(count: usize) -> String {
    let mut text = String::new();
    for n in 1..=count {
        writeln!(text, "{n}. Step.").expect("a string takes text");
    }
    text
}

fn folder(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[test]
fn an_outcome_with_more_items_than_children_is_noted() {
    let outcome = "## Outcome\n\n- One.\n* Two.\n3. Three.\n";
    let epic = issue("E1", "Epic", "", outcome);
    let child = task("T1", "parent = \"E1\"\n", "## Outcome\n\nO.\n");
    assert_eq!(
        lines(vec![epic, child]),
        ["E1.md: Outcome has 3 items and 1 children. Is an item covered by no task?"]
    );
}

#[test]
fn an_outcome_without_more_items_than_children_is_not_noted() {
    let epic = issue("E1", "Epic", "", "## Outcome\n\n- One.\n");
    let child = task("T1", "parent = \"E1\"\n", "");
    assert_eq!(lines(vec![epic, child]), Vec::<String>::new());
}

#[test]
fn an_epic_without_children_is_not_noted() {
    let epic = issue("E1", "Epic", "", "## Outcome\n\n- One.\n- Two.\n");
    assert_eq!(lines(vec![epic]), Vec::<String>::new());
}

#[test]
fn a_task_with_eleven_steps_is_noted() {
    let steps = numbered(11);
    let body = format!("## Steps\n\n{steps}");
    assert_eq!(
        lines(vec![task("T1", "", &body)]),
        ["T1.md: 11 steps. Split a task with more than 10 steps"]
    );
}

#[test]
fn a_task_with_ten_steps_is_not_noted() {
    let steps = numbered(10);
    let body = format!("## Steps\n\n{steps}");
    assert_eq!(lines(vec![task("T1", "", &body)]), Vec::<String>::new());
}

#[test]
fn a_task_without_a_builder_is_noted() {
    let text = ROUTED.replace("agent = \"a\"\n", "");
    let bare = issue("T1", "Task", &text, "");
    assert_eq!(lines(vec![bare]), ["T1.md: no builder"]);
}

#[test]
fn a_bug_without_a_reviewer_is_noted() {
    let text = ROUTED.replace("review_agent = \"e\"\n", "");
    let bug = issue("B1", "Bug", &text, "");
    assert_eq!(lines(vec![bug]), ["B1.md: no reviewer"]);
}

#[test]
fn an_owner_task_has_no_builder_or_reviewer_note() {
    let owner = issue("T1", "Task", "agent = \"Owner\"\n", "");
    assert_eq!(lines(vec![owner]), Vec::<String>::new());
}

#[test]
fn a_reviewer_with_the_model_of_the_builder_is_noted() {
    let text = ROUTED.replace("review_model = \"g\"", "review_model = \"c\"");
    let twin = issue("T1", "Task", &text, "");
    assert_eq!(
        lines(vec![twin]),
        ["T1.md: the reviewer uses the builder's model c"]
    );
}

#[test]
fn notes_are_sorted_by_file_then_message() {
    let text = ROUTED
        .replace("agent = \"a\"\n", "")
        .replace("review_agent = \"e\"\n", "");
    let second = issue("B2", "Task", &text, "");
    let first = issue("A1", "Task", &text, "");
    assert_eq!(
        lines(vec![second, first]),
        [
            "A1.md: no builder",
            "A1.md: no reviewer",
            "B2.md: no builder",
            "B2.md: no reviewer"
        ]
    );
}

#[test]
fn the_good_draft_has_no_note() {
    let draft = read(&folder("draft-good")).expect("draft-good reads");
    assert_eq!(notes(&draft).len(), 0);
    let report = run(&folder("draft-good"), &config::default()).expect("the report runs");
    assert_eq!(report, "COVERAGE 0 notes");
}

#[test]
fn a_missing_folder_is_an_error() {
    assert!(run(&folder("no-such-draft"), &config::default()).is_err());
}
