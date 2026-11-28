//! Tests of the project markers and the organization listing.

use super::{Marker, kind_of, list, marker_line, parse_marker, with_marker};
use crate::config::ProjectKind;
use crate::gh::FakeGh;
use crate::link::ProjectLink;

fn marker(key: &str, kind: ProjectKind) -> Marker {
    Marker {
        key: key.to_owned(),
        kind,
    }
}

fn project(number: u64) -> ProjectLink {
    ProjectLink {
        owner: "tailrocks".to_owned(),
        number,
    }
}

fn readme_answer(readme: &str) -> String {
    serde_json::json!({"data": {"organization": {"projectV2": {"readme": readme}}}}).to_string()
}

#[test]
fn marker_line_and_parse_marker_round_trip() {
    for kind in [ProjectKind::Delivery, ProjectKind::Portfolio] {
        let want = marker("taskrock", kind);
        let line = marker_line(&want);
        assert_eq!(
            parse_marker(&format!("About it.\n\n{line}\n")),
            Some(want),
            "{line}"
        );
    }
    assert_eq!(
        marker_line(&marker("roadmap", ProjectKind::Portfolio)),
        "<!-- taskrock:project key=roadmap kind=portfolio -->"
    );
}

#[test]
fn parse_marker_takes_the_last_marker_line() {
    let readme = format!(
        "{}\ntext\n{}",
        marker_line(&marker("a", ProjectKind::Delivery)),
        marker_line(&marker("b", ProjectKind::Portfolio)),
    );
    assert_eq!(
        parse_marker(&readme),
        Some(marker("b", ProjectKind::Portfolio))
    );
}

#[test]
fn readme_without_marker_gives_none() {
    assert_eq!(parse_marker(""), None);
    assert_eq!(parse_marker("Just a README.\n<!-- a comment -->"), None);
}

#[test]
fn wrong_kind_word_gives_none() {
    assert_eq!(
        parse_marker("<!-- taskrock:project key=x kind=product -->"),
        None
    );
    assert_eq!(parse_marker("<!-- taskrock:project key=x -->"), None);
}

#[test]
fn with_marker_replaces_an_old_marker() {
    let old = marker_line(&marker("old", ProjectKind::Delivery));
    let new = marker("new", ProjectKind::Portfolio);
    let readme = format!("Title\n\n{old}\n");
    let out = with_marker(&readme, &new);
    assert_eq!(out, format!("Title\n\n{}", marker_line(&new)));
    assert_eq!(with_marker(&out, &new), out);
    assert_eq!(with_marker("", &new), marker_line(&new));
}

#[test]
fn list_returns_open_projects_with_their_markers() {
    let line = marker_line(&marker("roadmap", ProjectKind::Portfolio));
    let body = serde_json::json!({"data": {"organization": {"projectsV2": {"nodes": [
        {"id": "P1", "number": 3, "title": "taskrock", "readme": null, "public": false, "closed": false},
        {"id": "P2", "number": 4, "title": "Roadmap", "readme": format!("Plan\n\n{line}"), "public": true, "closed": false},
        {"id": "P3", "number": 5, "title": "Old", "readme": "", "public": false, "closed": true},
    ]}}}})
    .to_string();
    let gh = FakeGh::new().ok("graphql", &body);
    let projects = list(&gh, "tailrocks").expect("list");
    let numbers: Vec<u64> = projects.iter().map(|found| found.number).collect();
    assert_eq!(numbers, [3, 4]);
    assert_eq!(projects[0].marker, None);
    assert_eq!(
        projects[1].marker,
        Some(marker("roadmap", ProjectKind::Portfolio))
    );
    assert!(projects[1].public);
}

#[test]
fn kind_of_reads_the_marker_and_defaults_to_delivery() {
    let line = marker_line(&marker("roadmap", ProjectKind::Portfolio));
    let gh = FakeGh::new().ok("graphql", &readme_answer(&line));
    assert_eq!(
        kind_of(&gh, &project(4)).expect("kind"),
        ProjectKind::Portfolio
    );
    let gh = FakeGh::new().ok("graphql", &readme_answer("No marker here."));
    assert_eq!(
        kind_of(&gh, &project(3)).expect("kind"),
        ProjectKind::Delivery
    );
}

/// The JSON body that the one call sent to `gh api graphql --input -`.
fn sent(gh: &FakeGh) -> serde_json::Value {
    let calls = gh.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "api graphql --input -");
    serde_json::from_str(calls[0].1.as_deref().expect("input")).expect("json")
}

#[test]
fn kind_of_sends_the_organization_query_with_a_numeric_number() {
    let gh = FakeGh::new().ok("graphql", &readme_answer(""));
    kind_of(&gh, &project(4)).expect("kind");
    let sent = sent(&gh);
    let query = sent["query"].as_str().expect("query");
    assert!(query.contains("organization(login:$o){projectV2(number:$n)"));
    assert_eq!(
        sent["variables"],
        serde_json::json!({"o": "tailrocks", "n": 4})
    );
}

#[test]
fn list_sends_the_organization_query_with_the_owner() {
    let gh = FakeGh::new().ok(
        "graphql",
        r#"{"data":{"organization":{"projectsV2":{"nodes":[]}}}}"#,
    );
    list(&gh, "tailrocks").expect("list");
    let sent = sent(&gh);
    let query = sent["query"].as_str().expect("query");
    assert!(query.contains("organization(login:$o){projectsV2(first:100)"));
    assert_eq!(sent["variables"], serde_json::json!({"o": "tailrocks"}));
}
