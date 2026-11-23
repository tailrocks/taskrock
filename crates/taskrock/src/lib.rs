//! taskrock: plan and run agent work on GitHub Issues and Projects.

/// The version of taskrock, from the workspace manifest.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
