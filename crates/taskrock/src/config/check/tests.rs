use super::{check, is_date};
use crate::config::parse::parse;
use crate::config::{Config, SharedMilestone};

const GOOD: &str = include_str!("../../../fixtures/config-good.toml");

fn good() -> Config {
    parse(GOOD).expect("the good fixture parses")
}
fn with(change: impl FnOnce(&mut Config)) -> Vec<String> {
    let mut config = good();
    change(&mut config);
    check(&config)
}
fn milestone(title: &str, due: Option<&str>) -> SharedMilestone {
    SharedMilestone {
        title: title.to_owned(),
        due: due.map(str::to_owned),
        description: None,
    }
}

#[test]
fn rule_1_version() {
    assert_eq!(with(|c| c.version = 2), ["version must be 1"]);
}
#[test]
fn rule_2_duplicate_names() {
    let found = with(|c| {
        c.routing.subagents.push(c.routing.subagents[0].clone());
        c.routing.models.push(c.routing.models[0].clone());
        c.issue_types.push(c.issue_types[0].clone());
        c.projects.push(c.projects[0].clone());
    });
    let names = "subagent builder,model small,issue type Initiative,project main".split(',');
    assert_eq!(
        found,
        names
            .map(|n| format!("{n} is listed twice"))
            .collect::<Vec<_>>()
    );
}
#[test]
fn rule_3_runtime_not_listed() {
    let found = with(|c| {
        c.routing.subagents[0].runtimes.push("grok".to_owned());
        c.routing.models[0].runtimes.push("muse".to_owned());
    });
    assert_eq!(
        found,
        [
            "subagent builder names runtime grok, which is not in routing.runtimes",
            "model small names runtime muse, which is not in routing.runtimes"
        ]
    );
}
#[test]
fn rules_4_to_6_empty_can_no_efforts_max_rounds() {
    let found = with(|c| {
        c.routing.subagents[1].can.clear();
        c.routing.models[0].efforts.clear();
        c.review.max_rounds = 0;
    });
    assert_eq!(found[0], "subagent reviewer has an empty can");
    assert_eq!(found[1], "model small has no efforts");
    assert_eq!(found[2], "review.max_rounds must be at least 1");
}
#[test]
fn rule_7_missing_type() {
    assert_eq!(
        with(|c| c.issue_types.truncate(4)),
        ["issue type Feature is missing"]
    );
}
#[test]
fn rules_8_and_9_sections_of_a_type() {
    let found = with(|c| {
        c.issue_types[2].format.file.push("Extra".to_owned());
        c.issue_types[2].format.checklist.push("Other".to_owned());
    });
    assert_eq!(
        found,
        [
            "issue type Task: file section Extra is not a build section",
            "issue type Task: checklist section Other is not a build section"
        ]
    );
}
#[test]
fn rule_10_sections_of_the_owner_task() {
    let found = with(|c| c.owner_task.file.push("Extra".to_owned()));
    assert_eq!(
        found,
        ["owner task: file section Extra is not a build section"]
    );
    let found = with(|c| c.owner_task.checklist.push("Other".to_owned()));
    assert_eq!(
        found,
        ["owner task: checklist section Other is not a build section"]
    );
}
#[test]
fn rule_11_delivery_needs_a_repo() {
    let found = with(|c| c.projects[0].repos.clear());
    assert_eq!(found, ["project main is a delivery project with no repos"]);
}
#[test]
fn rule_12_portfolio_has_no_repos_and_no_milestones() {
    let found = with(|c| {
        c.projects[1].repos.push("app".to_owned());
        c.projects[1].milestones.push(milestone("v1", None));
    });
    assert_eq!(
        found,
        [
            "project roadmap is a portfolio project, so it has no repos",
            "project roadmap is a portfolio project, so it has no milestones"
        ]
    );
}
#[test]
fn rule_13_duplicate_milestone() {
    let found = with(|c| c.projects[0].milestones.push(milestone("v0.2.0", None)));
    assert_eq!(found, ["project main: milestone v0.2.0 is listed twice"]);
}
#[test]
fn rule_14_due_is_a_date() {
    let found = with(|c| c.projects[0].milestones[0].due = Some("2026-02-30".to_owned()));
    assert_eq!(
        found,
        ["project main: milestone v0.2.0 has due 2026-02-30, which is not a date YYYY-MM-DD"]
    );
    for ok in "2024-02-29 2000-02-29 2026-12-31".split(' ') {
        assert!(is_date(ok), "{ok}");
    }
    for bad in "2100-02-29 2026-13-01 2026-00-10 2026-04-31 2026-1-01 x 2026-01-00".split(' ') {
        assert!(!is_date(bad), "{bad}");
    }
}
#[test]
fn rule_15_project_key_characters() {
    let found = with(|c| c.projects[0].key = "my_key".to_owned());
    assert_eq!(
        found,
        ["project key my_key may have only letters, digits and -"]
    );
}
