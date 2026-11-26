//! Tests of views that exist: filter, columns, layout, extra views, errors.

use serde_json::json;

use super::{answer, complete_views, existing, fake, input, run_step, view};
use crate::config::ProjectKind::Portfolio;

const TABLE: &str = "TABLE_LAYOUT";
const FILTER: &str = "type:Initiative";
const ROADMAP: [&str; 3] = ["Title", "Status", "Sub-issues progress"];

#[test]
fn a_filter_that_differs_is_fixed() {
    let initiatives = [
        "Title",
        "Status",
        "Sub-issues progress",
        "Start date",
        "Target date",
    ];
    let views = [
        view("Initiatives", TABLE, "type:Epic", &initiatives),
        view("Roadmap", "ROADMAP_LAYOUT", FILTER, &ROADMAP),
    ];
    let gh = fake(&answer(None, &views), 1);
    let (result, lines) = run_step(&gh, &existing(Portfolio), false);
    result.expect("step");
    assert_eq!(
        lines,
        ["update view Initiatives (filter)", "exists view Roadmap"]
    );
    assert_eq!(
        input(&gh, 1),
        json!({"viewId": "V_Initiatives", "filter": FILTER})
    );
}

#[test]
fn a_missing_column_is_added_after_the_visible_ones() {
    let shown = [
        "Title",
        "Assignees",
        "Status",
        "Sub-issues progress",
        "Start date",
    ];
    let views = [
        view("Initiatives", TABLE, FILTER, &shown),
        view("Roadmap", "ROADMAP_LAYOUT", FILTER, &ROADMAP),
    ];
    let gh = fake(&answer(None, &views), 1);
    let (result, lines) = run_step(&gh, &existing(Portfolio), false);
    result.expect("step");
    assert_eq!(lines[0], "update view Initiatives (columns)");
    let wanted = [&shown[..], &["Target date"]].concat();
    assert_eq!(
        input(&gh, 1),
        json!({"viewId": "V_Initiatives", "configuration": {"visibleFieldIds": wanted}})
    );
}

#[test]
fn a_layout_that_differs_is_noted_and_left() {
    let mut views = complete_views(Portfolio);
    views[1] = view("Roadmap", TABLE, FILTER, &ROADMAP);
    let gh = fake(&answer(None, &views), 0);
    let (result, lines) = run_step(&gh, &existing(Portfolio), false);
    result.expect("step");
    assert_eq!(
        lines[1],
        "view Roadmap has layout table, wanted roadmap. The API cannot change it"
    );
    assert_eq!(gh.calls().len(), 1);
}

#[test]
fn an_extra_view_is_noted_and_kept() {
    let mut views = complete_views(Portfolio);
    views.push(view("Old", TABLE, "", &[]));
    let gh = fake(&answer(None, &views), 0);
    let (result, lines) = run_step(&gh, &existing(Portfolio), false);
    result.expect("step");
    assert_eq!(lines.last().expect("line"), "extra view Old");
    assert_eq!(lines.len(), 3);
    assert_eq!(gh.calls().len(), 1);
}

#[test]
fn a_column_that_is_not_a_field_is_an_error() {
    let fields = ["Title", "Status", "Sub-issues progress", "Start date"];
    let gh = fake(&answer(Some(&fields), &[]), 0);
    let error = run_step(&gh, &existing(Portfolio), false)
        .0
        .expect_err("no field");
    assert_eq!(
        error.to_string(),
        "view Initiatives: the project has no field Target date"
    );
}
