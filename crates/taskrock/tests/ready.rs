//! `taskrock ready` against the project fixtures.

use taskrock::gh::FakeGh;
use taskrock::ready::run;

const FIELDS: &str = include_str!("../fixtures/project_fields.json");
const ITEMS: &str = include_str!("../fixtures/project_items.json");

const PROJECT: &str = "https://github.com/orgs/tailrocks/projects/3";

fn readme_answer(readme: &str) -> String {
    serde_json::json!({"data": {"repositoryOwner": {"projectV2": {"readme": readme}}}}).to_string()
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
    let text = run(&project_gh(), PROJECT, false, false).expect("ready text");
    assert!(text.starts_with("READY "), "{text}");
    let out = run(&project_gh(), PROJECT, true, false).expect("ready json");
    let json: serde_json::Value = serde_json::from_str(&out).expect("json parses");
    for key in [
        "ready",
        "waiting_on_owner",
        "triage",
        "loose",
        "in_progress",
        "blocked",
        "problems",
    ] {
        assert!(json.get(key).is_some(), "json has {key}");
    }
    for entry in json["ready"].as_array().expect("ready is a list") {
        assert!(
            entry.get("stacked").is_some(),
            "ready entry has stacked: {entry}"
        );
    }
}

#[test]
fn run_refuses_a_portfolio_project() {
    let marker = "<!-- taskrock:project key=roadmap kind=portfolio -->";
    let gh = FakeGh::new().ok("graphql", &readme_answer(&format!("Plan\n\n{marker}")));
    let error = run(
        &gh,
        "https://github.com/orgs/tailrocks/projects/4",
        false,
        false,
    )
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
    let out = run(&gh, PROJECT, true, false).expect("ready json");
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

#[test]
fn run_json_lists_the_loose_issues_of_the_link_milestone() {
    // Item 99 is unblocked, so it would be ready if the scope reached it.
    let mut items: Vec<serde_json::Value> = serde_json::from_str(ITEMS).expect("fixture parses");
    for item in &mut items {
        if item["content"]["number"] == 99 {
            item["content"]["issue_dependencies_summary"]["blocked_by"] = serde_json::json!(0);
        }
    }
    let unblocked = serde_json::Value::Array(items).to_string();
    let projects = serde_json::json!({"data": {"repository": {"issue": {"projectItems": {
        "nodes": [{"project": {
            "number": 3,
            "readme": "No marker, so delivery.",
            "owner": {"login": "tailrocks"},
        }}],
    }}}}});
    let gh = FakeGh::new()
        .ok("graphql", &projects.to_string())
        .ok("3/fields", &format!("[{FIELDS}]"))
        .ok("3/items", &format!("[{unblocked}]"))
        .ok("matching-refs", "[[]]")
        .ok("issues?state=open", "[[]]")
        .fail("contents/taskrock.toml", "gh: Not Found (HTTP 404)");
    let epic = "https://github.com/tailrocks/taskrock/issues/5";
    let out = run(&gh, epic, true, false).expect("ready json");
    let json: serde_json::Value = serde_json::from_str(&out).expect("json parses");
    assert_eq!(
        json["loose"],
        serde_json::json!([{
            "number": 99,
            "title": "A task filed outside the tree",
            "url": "https://github.com/tailrocks/taskrock/issues/99",
            "type": "Task",
        }])
    );
    let listed = json["ready"].as_array().expect("ready is a list");
    assert!(listed.iter().all(|entry| entry["number"] != 99));
    let problems = json["problems"].to_string();
    assert!(!problems.contains("issues/99"), "{problems}");
}

#[test]
fn run_json_of_a_project_link_has_an_empty_loose_list() {
    let out = run(&project_gh(), PROJECT, true, false).expect("ready json");
    let json: serde_json::Value = serde_json::from_str(&out).expect("json parses");
    assert_eq!(json["loose"], serde_json::json!([]));
}
