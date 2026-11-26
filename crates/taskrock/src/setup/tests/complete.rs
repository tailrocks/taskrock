//! The fake answer of a project that needs no change.

/// The fields of a project that needs nothing: Status complete, every column.
pub(super) fn project_fields() -> String {
    let status = ["Triage", "Todo", "In Progress", "Done"].map(
        |name| serde_json::json!({"id": name, "name": name, "color": "GRAY", "description": ""}),
    );
    let mut nodes = vec![serde_json::json!({"id": "S", "name": "Status", "options": status})];
    let columns = [
        "Agent",
        "Review agent",
        "Runtime",
        "Review runtime",
        "Model",
    ]
    .into_iter()
    .chain([
        "Review model",
        "Effort",
        "Review effort",
        "Start date",
        "Target date",
    ]);
    nodes.extend(columns.map(|name| serde_json::json!({"id": name, "name": name})));
    serde_json::json!({"data": {"node": {"fields": {"nodes": nodes}}}}).to_string()
}
