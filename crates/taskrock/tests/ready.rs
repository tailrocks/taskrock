//! `taskrock ready` against the project fixtures.

use taskrock::gh::FakeGh;
use taskrock::ready::run;

const FIELDS: &str = include_str!("../fixtures/project_fields.json");
const ITEMS: &str = include_str!("../fixtures/project_items.json");

const PROJECT: &str = "https://github.com/orgs/tailrocks/projects/3";

fn readme_answer(readme: &str) -> String {
    serde_json::json!({"data": {"organization": {"projectV2": {"readme": readme}}}}).to_string()
}

fn project_gh() -> FakeGh {
    FakeGh::new()
        .ok("graphql", &readme_answer("No marker, so delivery."))
        .ok("3/fields", &format!("[{FIELDS}]"))
        .ok("3/items", &format!("[{ITEMS}]"))
        .ok("matching-refs", "[[]]")
        .ok("issues?state=open", "[[]]")
        .fail("contents/taskrock.toml", "gh: Not Found (HTTP 404)")
}

#[test]
fn run_lists_the_project_as_text_and_json() {
    let text = run(&project_gh(), PROJECT, false).expect("ready text");
    assert!(text.starts_with("READY "), "{text}");
    let out = run(&project_gh(), PROJECT, true).expect("ready json");
    let json: serde_json::Value = serde_json::from_str(&out).expect("json parses");
    for key in [
        "ready",
        "waiting_on_owner",
        "in_progress",
        "blocked",
        "problems",
    ] {
        assert!(json.get(key).is_some(), "json has {key}");
    }
}

#[test]
fn run_refuses_a_portfolio_project() {
    let marker = "<!-- taskrock:project key=roadmap kind=portfolio -->";
    let gh = FakeGh::new().ok("graphql", &readme_answer(&format!("Plan\n\n{marker}")));
    let error = run(&gh, "https://github.com/orgs/tailrocks/projects/4", false)
        .expect_err("portfolio is refused");
    assert!(
        matches!(&error, taskrock::error::Error::Refused(_)),
        "{error}"
    );
    assert_eq!(
        error.to_string(),
        "https://github.com/orgs/tailrocks/projects/4 is a portfolio project. Give a delivery project or an initiative"
    );
    assert_eq!(gh.calls().len(), 1, "no item is read");
}
