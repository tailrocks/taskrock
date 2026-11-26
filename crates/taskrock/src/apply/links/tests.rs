//! Tests of the linked issues.

use serde_json::json;

use crate::apply::links::resolve;
use crate::draft;
use crate::error::Error;
use crate::gh::FakeGh;

use super::super::tests::MISSING;

pub(crate) const FIVE: &str = "https://github.com/tailrocks/holla/issues/5";
pub(crate) const SIX: &str = "https://github.com/tailrocks/holla/issues/6";

pub(crate) fn links_folder() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/draft-links")
}

fn draft() -> draft::Draft {
    draft::parse::read(&links_folder()).expect("fixture is a draft")
}

#[test]
fn each_link_is_read_once_by_its_api_path() {
    let gh = FakeGh::new()
        .ok("repos/tailrocks/holla/issues/5", r#"{"number":5,"id":55}"#)
        .ok("repos/tailrocks/holla/issues/6", r#"{"number":6,"id":66}"#);
    let found = resolve(&gh, &draft()).expect("links resolve");
    assert_eq!(found[FIVE].id, 55);
    assert_eq!(found[SIX].id, 66);
    assert_eq!(found[SIX].link.number, 6);
    assert_eq!(gh.calls().len(), 2);
}

#[test]
fn missing_issue_stops_with_its_link() {
    let gh = FakeGh::new().fail("issues/5", MISSING);
    let Err(Error::Apply(message)) = resolve(&gh, &draft()) else {
        panic!("expected Error::Apply");
    };
    assert_eq!(message, format!("no issue {FIVE}"));
}

#[test]
fn pull_request_is_not_an_issue() {
    let body = json!({"id": 55, "pull_request": {}}).to_string();
    let gh = FakeGh::new().ok("issues/5", &body);
    let Err(Error::Apply(message)) = resolve(&gh, &draft()) else {
        panic!("expected Error::Apply");
    };
    assert_eq!(message, format!("{FIVE} is a pull request, not an issue"));
}
