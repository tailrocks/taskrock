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

/// The replies of the repositories, milestones and workflows steps for
/// `config-good.toml`: every link, milestone and workflow is there already.
pub(super) fn linked_and_dated(gh: crate::gh::FakeGh) -> crate::gh::FakeGh {
    let linked = |names: &[&str]| {
        let nodes: Vec<_> = names
            .iter()
            .map(|name| serde_json::json!({"name": name}))
            .collect();
        serde_json::json!({"data": {"node": {"repositories": {"nodes": nodes}}}}).to_string()
    };
    let milestones =
        r#"[{"number":1,"title":"v0.2.0","description":null,"due_on":"2026-12-15T00:00:00Z"}]"#;
    gh.ok(
        "graphql",
        &linked(&["taskrock", "taskrock-plugins", "taskrock-sandbox"]),
    )
    .ok("graphql", &linked(&[]))
    .ok("repos/acme/taskrock/milestones", milestones)
    .ok("repos/acme/taskrock-plugins/milestones", milestones)
    .ok("repos/acme/taskrock-sandbox/milestones", milestones)
    .ok("graphql", &project_workflows())
    .ok("graphql", &project_workflows())
}

/// The workflows of a project that needs nothing: the six on, no view.
fn project_workflows() -> String {
    let nodes: Vec<_> = crate::setup::workflows::NAMES
        .iter()
        .map(|name| serde_json::json!({"id": name, "name": name, "enabled": true}))
        .collect();
    serde_json::json!({"data": {"node": {"workflows": {"nodes": nodes}, "views": {"nodes": []}}}})
        .to_string()
}
