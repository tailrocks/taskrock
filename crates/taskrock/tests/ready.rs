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
        "triage",
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

#[test]
fn run_json_lists_triage_items_outside_ready_and_problems() {
    let mut items: Vec<serde_json::Value> = serde_json::from_str(ITEMS).expect("fixture parses");
    for item in &mut items {
        let url = item["content"]["html_url"].as_str().unwrap_or_default();
        if url.ends_with("/issues/18") {
            item["fields"][0]["value"]["name"]["raw"] = serde_json::json!("Triage");
        }
    }
    let triage_items = serde_json::Value::Array(items).to_string();
    let gh = FakeGh::new()
        .ok("graphql", &readme_answer("No marker, so delivery."))
        .ok("3/fields", &format!("[{FIELDS}]"))
        .ok("3/items", &format!("[{triage_items}]"))
        .ok("matching-refs", "[[]]")
        .ok("issues?state=open", "[[]]")
        .fail("contents/taskrock.toml", "gh: Not Found (HTTP 404)");
    let out = run(&gh, PROJECT, true).expect("ready json");
    let json: serde_json::Value = serde_json::from_str(&out).expect("json parses");
    assert_eq!(
        json["triage"],
        serde_json::json!([{
            "number": 18,
            "title": json["triage"][0]["title"],
            "url": "https://github.com/tailrocks/taskrock/issues/18",
            "type": "Task",
        }])
    );
    let listed = json["ready"].as_array().expect("ready is a list");
    assert!(listed.iter().all(|entry| entry["number"] != 18));
    let problems = json["problems"].to_string();
    assert!(!problems.contains("issues/18"), "{problems}");
}
