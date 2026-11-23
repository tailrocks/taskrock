//! The marker line that links a GitHub issue to its draft.

/// The marker that ends the body of the issue created from `key`.
#[must_use]
pub(super) fn marker(plan_id: &str, key: &str) -> String {
    format!("<!-- taskrock:{plan_id}:{key} -->")
}

/// The draft key of the marker in `body` that names `plan_id`, if there is one.
#[must_use]
pub(super) fn marker_key(body: &str, plan_id: &str) -> Option<String> {
    let prefix = format!("<!-- taskrock:{plan_id}:");
    let start = body.find(&prefix)? + prefix.len();
    let rest = body[start..].trim_start();
    let end = rest.find(|got: char| !got.is_ascii_alphanumeric() && got != '-')?;
    let key = rest[..end].to_owned();
    let after = rest[end..].trim_start();
    if key.is_empty() || !after.starts_with("-->") {
        return None;
    }
    Some(key)
}
