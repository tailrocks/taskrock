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
