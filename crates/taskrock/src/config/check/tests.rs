use super::{check, is_date};
use crate::config::Config;
use crate::config::parse::parse;

const GOOD: &str = include_str!("../../../fixtures/config-good.toml");

fn text(word: &str) -> String {
    word.to_owned()
}
fn with(change: impl FnOnce(&mut Config)) -> Vec<String> {
    let mut config = parse(GOOD).expect("the good fixture parses");
    change(&mut config);
    check(&config)
}
/// A test named `$name`: the messages for the good fixture after `$change`.
macro_rules! rule {
    ($name:ident, $change:expr, $messages:expr) => {
        #[test]
        fn $name() {
            assert_eq!(with($change), $messages);
        }
    };
}

rule!(
    messages_come_in_file_order,
    |c| {
        c.routing.subagents[0].name = text("first");
        c.routing.subagents[0].can.clear();
        c.routing.subagents[1].name = text("second");
        c.routing.subagents[1].runtimes = vec![text("grok")];
        c.routing.runtimes.retain(|r| r != "grok");
        c.projects[0].key = text("a");
        c.projects[1] = c.projects[0].clone();
    },
    [
        "subagent first has an empty can",
        "subagent second names runtime grok, which is not in routing.runtimes",
        "project a is listed twice"
    ]
);
rule!(the_good_fixture_passes, |_| (), [] as [&str; 0]);
rule!(rule_1_version, |c| c.version = 2, ["version must be 1"]);
rule!(
    rule_2_duplicate_names,
    |c| {
        c.routing.subagents.push(c.routing.subagents[0].clone());
        c.routing.models.push(c.routing.models[0].clone());
        c.issue_types.push(c.issue_types[0].clone());
        c.projects.push(c.projects[0].clone());
    },
    [
        "subagent tailrocks-systems-engineer is listed twice",
        "model claude-haiku-5-5 is listed twice",
        "issue type Initiative is listed twice",
        "project taskrock is listed twice"
    ]
);
rule!(
    rule_3_runtime_not_listed,
    |c| {
        c.routing.subagents[0].runtimes.push(text("zed"));
        c.routing.models[0].runtimes.push(text("zed"));
    },
    [
        "subagent tailrocks-systems-engineer names runtime zed, which is not in routing.runtimes",
        "model claude-haiku-5-5 names runtime zed, which is not in routing.runtimes"
    ]
);
rule!(
    rule_4_subagent_can_do_something,
    |c| c.routing.subagents[1].can.clear(),
    ["subagent tailrocks-code-reviewer has an empty can"]
);
rule!(
    rule_5_model_has_an_effort,
    |c| c.routing.models[2].efforts.clear(),
    ["model claude-opus-5-5 has no efforts"]
);
rule!(
    rule_6_max_rounds,
    |c| c.review.max_rounds = 0,
    ["review.max_rounds must be at least 1"]
);
rule!(
    rule_7_missing_type,
    |c| c.issue_types.truncate(4),
    ["issue type Feature is missing"]
);
rule!(
    rule_8_file_section_is_a_build_section,
    |c| c.issue_types[2].format.file.push(text("Extra")),
    ["issue type Task: file section Extra is not a build section"]
);
rule!(
    rule_9_checklist_section_is_a_build_section,
    |c| c.issue_types[2].format.checklist.push(text("Other")),
    ["issue type Task: checklist section Other is not a build section"]
);
rule!(
    rule_10_sections_of_the_owner_task,
    |c| {
        c.owner_task.file.push(text("Extra"));
        c.owner_task.checklist.push(text("Other"));
    },
    [
        "owner task: file section Extra is not a build section",
        "owner task: checklist section Other is not a build section"
    ]
);
rule!(
    rule_11_delivery_needs_a_repo,
    |c| c.projects[0].repos.clear(),
    ["project taskrock is a delivery project with no repos"]
);
rule!(
    rule_12_portfolio_has_no_repos_and_no_milestones,
    |c| {
        c.projects[1].repos.push(text("app"));
        c.projects[1].milestones = c.projects[0].milestones.clone();
    },
    [
        "project roadmap is a portfolio project, so it has no repos",
        "project roadmap is a portfolio project, so it has no milestones"
    ]
);
rule!(
    rule_13_duplicate_milestone,
    |c| {
        let project = &mut c.projects[0];
        project.milestones.push(project.milestones[0].clone());
    },
    ["project taskrock: milestone v0.2.0 is listed twice"]
);
rule!(
    rule_14_due_is_a_date,
    |c| c.projects[0].milestones[0].due = Some(text("2026-02-30")),
    ["project taskrock: milestone v0.2.0 has due 2026-02-30, which is not a date YYYY-MM-DD"]
);
rule!(
    rule_15_project_key_characters,
    |c| c.projects[0].key = text("my_key"),
    ["project key my_key may have only letters, digits and -"]
);
#[test]
fn dates_respect_the_month_and_leap_years() {
    assert!("2024-02-29 2000-02-29 2026-12-31".split(' ').all(is_date));
    let bad = "2100-02-29 2026-13-01 2026-00-10 2026-04-31 2026-1-01 x 2026-01-00";
    assert!(!bad.split(' ').any(is_date));
}
