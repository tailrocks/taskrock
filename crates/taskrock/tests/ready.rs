//! `taskrock ready` against the project fixtures.

use taskrock::gh::FakeGh;
use taskrock::ready::run;

const FIELDS: &str = include_str!("../fixtures/project_fields.json");
const ITEMS: &str = include_str!("../fixtures/project_items.json");

const PROJECT: &str = "https://github.com/orgs/tailrocks/projects/3";

fn project_gh() -> FakeGh {
    FakeGh::new()
        .ok("3/fields", &format!("[{FIELDS}]"))
        .ok("3/items", &format!("[{ITEMS}]"))
        .ok("matching-refs", "[[]]")
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
