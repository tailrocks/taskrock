use super::check;
use crate::config;
use crate::draft::parse::{parse_issue, parse_plan, read};
use crate::draft::{Draft, DraftIssue};
use crate::fields::Routing;
pub(super) const BODY: &str = "## Outcome\n\nO.\n\n## Decisions already made\n\nD.\n\n## Not yet specified\n\nNone.\n\n## Steps\n\nS.\n\n## Done when\n\n- [ ] `d` prints ok.\n\n## Out of scope\n\nO.\n";
const PLAN: &str =
    "id = \"p\"\nrepo = \"o/n\"\nproject = \"https://github.com/orgs/o/projects/7\"\n";
pub(super) fn issue(key: &str, extra: &str) -> DraftIssue {
    let parent = if extra.contains("parent") {
        ""
    } else {
        "parent = \"https://github.com/tailrocks/holla/issues/1\"\n"
    };
    let text = format!(
        "+++\nkey = \"{key}\"\ntitle = \"{key}\"\ntype = \"Task\"\n{parent}agent = \"a\"\nruntime = \"b\"\nmodel = \"c\"\neffort = \"d\"\nreview_agent = \"e\"\nreview_runtime = \"f\"\nreview_model = \"g\"\nreview_effort = \"h\"\n{extra}\n+++\n{BODY}"
    );
    parse_issue(&format!("{key}.md"), &text, "tailrocks/holla").expect("the test issue parses")
}
pub(super) fn lines(issues: Vec<DraftIssue>) -> Vec<String> {
    let plan = parse_plan(PLAN).expect("the test plan parses");
    let draft = Draft { plan, issues };
    check(&draft, &config::default())
        .into_iter()
        .map(|f| format!("{}: {}", f.file, f.message))
        .collect()
}
pub(super) fn epic(key: &str) -> DraftIssue {
    let mut full = issue(key, "");
    full.kind = "Epic".to_owned();
    full.routing = Routing::default();
    full
}
fn rings(blocked: bool) {
    for size in 1..=3usize {
        let keys: Vec<String> = (0..size).map(|n| format!("K{n}")).collect();
        let mut issues = Vec::new();
        for (n, key) in keys.iter().enumerate() {
            let mut full = if blocked { issue(key, "") } else { epic(key) };
            let next = keys[(n + 1) % size].clone();
            if blocked {
                full.blocked_by = vec![next];
            } else {
                full.parent = Some(next);
            }
            issues.push(full);
        }
        issues.extend((!blocked).then(|| issue("D", "parent = \"K0\"")));
        let word = if blocked { "blockers" } else { "parents" };
        let expected = format!("K0.md: {word} form a cycle: {} -> K0", keys.join(" -> "));
        assert_eq!(lines(issues), [expected]);
    }
}
#[test]
fn good_draft_has_no_finding() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/draft-good");
    assert_eq!(
        check(&read(&dir).expect("draft-good reads"), &config::default()).len(),
        0
    );
}
#[test]
fn bad_keys_and_types_are_reported() {
    let mut bad = issue("S", "");
    bad.kind = "Story".to_owned();
    let mut dup = issue("S", "");
    dup.file = "b.md".to_owned();
    let found = lines(vec![bad, dup]);
    assert_eq!(found.len(), 3);
    assert!(found[0].contains("S has no children, so its type must be Task, Bug or Feature"));
    assert!(found[1].contains("S.md: type must be one of Initiative, Epic, Task, Bug, Feature"));
    assert!(found[2].contains("b.md: key S is also used in S.md"));
}
#[test]
fn unknown_parent_is_reported() {
    let found = lines(vec![issue("T", "parent = \"N\"")]);
    assert_eq!(found, ["T.md: parent N is not a key in this draft"]);
}
#[test]
fn unknown_blocker_is_reported() {
    let found = lines(vec![issue("T", "blocked_by = [\"N\"]")]);
    assert_eq!(found, ["T.md: blocker N is not a key in this draft"]);
}
#[test]
fn unknown_milestone_is_reported() {
    let found = lines(vec![issue("T", "milestone = \"M9\"")]);
    assert_eq!(found, ["T.md: milestone M9 is not in plan.toml"]);
}
#[test]
fn cycles_cover_one_two_and_three_issues() {
    rings(true);
    rings(false);
}
#[test]
fn families_have_limits() {
    let mut many = vec![epic("E")];
    many.extend((0..100).map(|n| issue(&format!("C{n}"), "parent = \"E\"")));
    let mut parent = "E".to_owned();
    for n in 1..=7 {
        let key = format!("P{n}");
        let mut next = epic(&key);
        next.parent = Some(parent);
        many.push(next);
        parent = key;
    }
    many.push(issue("T", &format!("parent = \"{parent}\"")));
    let found = lines(many);
    assert_eq!(found.len(), 2);
    assert!(found[0].contains("E.md: E has 101 children"));
    assert!(found[1].contains("T is 9 levels deep"));
}

#[test]
fn a_bare_issue_number_is_reported() {
    let mut task = issue("T", "");
    task.body = BODY.replace("O.\n", "Fixes #12.\n");
    let found = lines(vec![task]);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found
            .iter()
            .all(|line| line.contains("#12 is a bare issue number. Name the key instead"))
    );
}

#[test]
fn bodies_and_titles_have_limits() {
    let mut big = issue("B", "");
    big.body = format!("{BODY}{}", "x".repeat(60_000));
    let mut empty = issue("T", "");
    empty.title = String::new();
    let found = lines(vec![big, empty]);
    assert_eq!(found.len(), 2);
    assert!(found[0].contains("B.md: the body has 60"));
    assert!(found[1].contains("T.md: the title must have 1 to 256"));
}
