use super::parse_plan;
use crate::error::Error;

const PLAN: &str =
    "id = \"p\"\nrepo = \"o/n\"\nproject = \"https://github.com/orgs/o/projects/7\"\n";

#[test]
fn portfolio_is_optional() {
    let plan = parse_plan(PLAN).expect("the plan parses");
    assert_eq!(plan.portfolio, None);
}

#[test]
fn portfolio_is_the_project_key() {
    let plan = parse_plan(&format!("{PLAN}portfolio = \"roadmap\"\n")).expect("the plan parses");
    assert_eq!(plan.portfolio.as_deref(), Some("roadmap"));
}

#[test]
fn portfolio_is_checked_like_the_id() {
    let Err(Error::Draft { file, message }) = parse_plan(&format!("{PLAN}portfolio = \"a b\"\n"))
    else {
        panic!("expected Error::Draft");
    };
    assert_eq!(file, "plan.toml");
    assert_eq!(
        message,
        "only letters, digits and - are allowed in portfolio"
    );
}

#[test]
fn milestone_is_optional_and_not_empty() {
    assert_eq!(parse_plan(PLAN).expect("plan").milestone, None);
    let plan = parse_plan(&format!("{PLAN}milestone = \"M2 Next\"\n")).expect("plan");
    assert_eq!(plan.milestone.as_deref(), Some("M2 Next"));
    let Err(Error::Draft { message, .. }) = parse_plan(&format!("{PLAN}milestone = \"\"\n")) else {
        panic!("expected Error::Draft");
    };
    assert_eq!(message, "milestone must not be empty");
}
