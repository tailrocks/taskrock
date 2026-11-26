//! Tests of "View 1", of created views and of the dry run.

use serde_json::json;

use super::{answer, existing, fake, input, run_step, view};
use crate::config::ProjectKind::{Delivery, Portfolio};

const TABLE: &str = "TABLE_LAYOUT";
const INITIATIVES: [&str; 5] = [
    "Title",
    "Status",
    "Sub-issues progress",
    "Start date",
    "Target date",
];

#[test]
fn view_1_becomes_the_first_view() {
    let default = view("View 1", TABLE, "", &["Title", "Assignees", "Status"]);
    let gh = fake(&answer(None, &[default]), 3);
    let (result, lines) = run_step(&gh, &existing(Portfolio), false);
    result.expect("step");
    assert_eq!(
        lines,
        [
            "update view View 1 (rename to Initiatives)",
            "create view Roadmap"
        ]
    );
    assert_eq!(
        input(&gh, 1),
        json!({"viewId": "V_View 1", "name": "Initiatives", "filter": "type:Initiative",
            "configuration": {"visibleFieldIds": INITIATIVES}})
    );
    assert_eq!(
        input(&gh, 2),
        json!({"projectId": "P_3", "name": "Roadmap", "layout": "ROADMAP_LAYOUT",
            "configuration": {"visibleFieldIds": ["Title", "Status", "Sub-issues progress"]}})
    );
    assert_eq!(
        input(&gh, 3),
        json!({"viewId": "V_new", "filter": "type:Initiative"})
    );
}

#[test]
fn view_1_gets_the_layout_of_a_board() {
    let default = view("View 1", TABLE, "", &["Title"]);
    let gh = fake(&answer(None, &[default]), 12);
    let (result, lines) = run_step(&gh, &existing(Delivery), false);
    result.expect("step");
    assert_eq!(lines[0], "update view View 1 (rename to Board)");
    assert_eq!(lines.len(), 7);
    assert_eq!(input(&gh, 1)["layout"], "BOARD_LAYOUT");
    assert_eq!(input(&gh, 2)["layout"], "TABLE_LAYOUT");
    assert_eq!(
        gh.calls().len(),
        13,
        "a view with no filter needs no update"
    );
}

#[test]
fn a_dry_run_writes_nothing_and_skips_unknown_columns() {
    let fields = ["Title", "Status", "Sub-issues progress", "Start date"];
    let gh = fake(&answer(Some(&fields), &[]), 0);
    let (result, lines) = run_step(&gh, &existing(Portfolio), true);
    result.expect("step");
    assert_eq!(lines, ["create view Initiatives", "create view Roadmap"]);
    assert_eq!(gh.calls().len(), 1);
}
