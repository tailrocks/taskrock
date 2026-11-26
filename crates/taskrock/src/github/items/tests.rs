//! Tests of the project items.

use crate::gh::FakeGh;
use crate::github::items::{project_items, set_status, status_field};
use crate::link::ProjectLink;

use super::StatusField;

const FIELDS: &str = include_str!("../../../fixtures/project_fields.json");
const ITEMS: &str = include_str!("../../../fixtures/project_items.json");

fn project() -> ProjectLink {
    ProjectLink {
        owner: "tailrocks".to_owned(),
        number: 3,
    }
}

fn fields_gh() -> FakeGh {
    FakeGh::new().ok("3/fields", &format!("[{FIELDS}]"))
}

fn items_gh() -> FakeGh {
    FakeGh::new()
        .ok("3/fields", &format!("[{FIELDS}]"))
        .ok("3/items", &format!("[{ITEMS}]"))
}

#[test]
fn status_field_returns_three_option_ids() {
    let field = status_field(&fields_gh(), &project()).expect("fields");
    assert_ne!(field.todo, "");
    assert_ne!(field.in_progress, "");
    assert_ne!(field.done, "");
    assert_eq!(field.triage, None, "the fixture has no Triage");
}

#[test]
fn project_items_returns_every_issue() {
    let items = project_items(&items_gh(), &project()).expect("items");
    let expected = serde_json::from_str::<Vec<serde_json::Value>>(ITEMS)
        .expect("fixture")
        .iter()
        .filter(|value| value["content_type"] == "Issue")
        .count();
    assert_eq!(items.len(), expected);
    assert!(items.len() >= 23);
}

#[test]
fn scaffold_item_has_kind_parent_and_agent() {
    let items = project_items(&items_gh(), &project()).expect("items");
    let item = items
        .iter()
        .find(|item| item.title == "Scaffold the repository with green generated CI")
        .expect("scaffold");
    assert_eq!(item.kind.as_deref(), Some("Task"));
    assert!(item.parent.is_some());
    assert_eq!(
        item.routing.get("Agent"),
        Some("tailrocks-systems-engineer")
    );
}

#[test]
fn initiative_item_has_no_parent() {
    let items = project_items(&items_gh(), &project()).expect("items");
    let item = items
        .iter()
        .find(|item| item.kind.as_deref() == Some("Initiative"))
        .expect("initiative");
    assert_eq!(item.parent, None);
}

#[test]
fn draft_issue_is_skipped() {
    let page = r#"{"id":1,"content_type":"DraftIssue","content":{},"fields":[]}"#;
    let gh = fields_gh().ok("3/items", &format!("[[{page}]]"));
    let items = project_items(&gh, &project()).expect("items");
    assert!(items.is_empty(), "draft issues are skipped");
}

#[test]
fn null_status_value_gives_none() {
    let content = r#"{"html_url":"https://github.com/o/r/issues/1","id":5,"title":"T","state":"open","type":null,"sub_issues_summary":{"total":0},"issue_dependencies_summary":{"blocked_by":0,"blocking":0},"issue_field_values":[]}"#;
    let page = format!(
        r#"{{"id":9,"content_type":"Issue","content":{content},"fields":[{{"name":"Status","value":null}}]}}"#
    );
    let gh = fields_gh().ok("3/items", &format!("[[{page}]]"));
    let items = project_items(&gh, &project()).expect("items");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].status, None);
}

#[test]
fn set_status_sends_patch_with_field_and_option() {
    let gh = FakeGh::new().ok("items/314", "");
    let field = StatusField {
        field_id: 7,
        todo: "t".to_owned(),
        in_progress: "p".to_owned(),
        done: "d".to_owned(),
        triage: None,
    };
    set_status(&gh, &project(), &field, 314, "t").expect("set");
    assert_eq!(
        gh.calls(),
        [(
            "api -X PATCH orgs/tailrocks/projectsV2/3/items/314 --input -".to_owned(),
            Some(r#"{"fields":[{"id":7,"value":"t"}]}"#.to_owned()),
        )]
    );
}
