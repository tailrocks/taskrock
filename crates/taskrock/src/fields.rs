//! The routing fields of an issue.

use std::collections::BTreeMap;

#[cfg(test)]
mod tests;

/// The eight routing fields, in the order GitHub shows them.
pub const ROUTING: [&str; 8] = [
    "Agent",
    "Runtime",
    "Model",
    "Effort",
    "Review agent",
    "Review runtime",
    "Review model",
    "Review effort",
];

/// The Agent value for work only the owner can do.
pub const OWNER: &str = "Owner";

/// The routing field values of one issue, by field name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Routing(BTreeMap<String, String>);

impl Routing {
    /// Set the value of a routing field.
    pub fn set(&mut self, field: &str, value: &str) {
        self.0.insert(field.to_owned(), value.to_owned());
    }

    /// The value of a routing field, if it has one.
    #[must_use]
    pub fn get(&self, field: &str) -> Option<&str> {
        self.0.get(field).map(String::as_str)
    }

    /// True when Agent is `Owner`.
    #[must_use]
    pub fn is_owner(&self) -> bool {
        self.get("Agent").is_some_and(|value| value == OWNER)
    }

    /// The names in `ROUTING` that have no value, in `ROUTING` order.
    #[must_use]
    pub fn missing(&self) -> Vec<&'static str> {
        ROUTING
            .into_iter()
            .filter(|field| self.get(field).is_none_or(str::is_empty))
            .collect()
    }
}

/// The key a draft uses for a field: lower case, spaces as `_`.
/// `Review agent` gives `review_agent`.
#[must_use]
pub fn draft_key(field: &str) -> String {
    field.to_lowercase().replace(' ', "_")
}
