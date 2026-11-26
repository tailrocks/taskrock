//! The text and JSON output of `taskrock ready`.

use crate::error::Error;
use crate::fields::{ROUTING, Routing, draft_key};

use super::Report;
use crate::stack::Stack;

fn show<'a>(routing: &'a Routing, field: &str) -> &'a str {
    routing
        .get(field)
        .filter(|value| !value.is_empty())
        .unwrap_or("?")
}

pub(super) fn text_of(report: &Report<'_>) -> String {
    let mut lines = vec![format!("READY {}", report.ready.len())];
    for item in &report.ready {
        lines.push(format!("{}  {}", item.link.url(), item.title));
        lines.push(format!(
            "  builds: {} / {} / {} / {}",
            show(&item.routing, "Agent"),
            show(&item.routing, "Runtime"),
            show(&item.routing, "Model"),
            show(&item.routing, "Effort"),
        ));
        lines.push(format!(
            "  reviews: {} / {} / {} / {}",
            show(&item.routing, "Review agent"),
            show(&item.routing, "Review runtime"),
            show(&item.routing, "Review model"),
            show(&item.routing, "Review effort"),
        ));
        if let Some(stack) = report.stacked.get(&item.link) {
            lines.push(format!(
                "  stacked on: {} {}",
                stack.base, stack.pull_request
            ));
        }
    }
    lines.push(format!("IN PROGRESS {}", report.in_progress));
    lines.push(format!("BLOCKED {}", report.blocked));
    lines.push(format!(
        "WAITING ON OWNER {}",
        report.waiting_on_owner.len()
    ));
    for item in &report.waiting_on_owner {
        lines.push(format!("{}  {}", item.link.url(), item.title));
    }
    lines.push(format!("TRIAGE {}", report.triage.len()));
    for item in &report.triage {
        let kind = item.kind.as_deref().unwrap_or("?");
        lines.push(format!("{}  {} ({kind})", item.link.url(), item.title));
    }
    lines.push(format!("PROBLEMS {}", report.problems.len()));
    lines.extend(report.problems.iter().cloned());
    lines.join("\n")
}

fn json_field(routing: &Routing, field: &str) -> serde_json::Value {
    match routing.get(field).filter(|value| !value.is_empty()) {
        Some(value) => serde_json::Value::String(value.to_owned()),
        None => serde_json::Value::Null,
    }
}

fn stacked_json(stack: Option<&Stack>) -> serde_json::Value {
    match stack {
        Some(stack) => serde_json::json!({
            "base": stack.base,
            "blocker": stack.blocker.number,
            "pull_request": stack.pull_request,
        }),
        None => serde_json::Value::Null,
    }
}

pub(super) fn json_of(report: &Report<'_>) -> Result<String, Error> {
    let mut ready = Vec::new();
    for item in &report.ready {
        let mut entry = serde_json::Map::new();
        entry.insert("url".to_owned(), serde_json::Value::String(item.link.url()));
        entry.insert(
            "number".to_owned(),
            serde_json::Value::from(item.link.number),
        );
        entry.insert(
            "title".to_owned(),
            serde_json::Value::String(item.title.clone()),
        );
        for field in ROUTING {
            entry.insert(draft_key(field), json_field(&item.routing, field));
        }
        entry.insert(
            "stacked".to_owned(),
            stacked_json(report.stacked.get(&item.link)),
        );
        ready.push(serde_json::Value::Object(entry));
    }
    let mut waiting = Vec::new();
    for item in &report.waiting_on_owner {
        waiting.push(serde_json::json!({
            "url": item.link.url(),
            "title": item.title,
        }));
    }
    let triage: Vec<serde_json::Value> = report
        .triage
        .iter()
        .map(|item| {
            serde_json::json!({
                "number": item.link.number,
                "title": item.title,
                "url": item.link.url(),
                "type": item.kind,
            })
        })
        .collect();
    let out = serde_json::json!({
        "ready": ready,
        "waiting_on_owner": waiting,
        "triage": triage,
        "in_progress": report.in_progress,
        "blocked": report.blocked,
        "problems": report.problems,
    });
    serde_json::to_string_pretty(&out).map_err(|error| Error::Json(error.to_string()))
}
