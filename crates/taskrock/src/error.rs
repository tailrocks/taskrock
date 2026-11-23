//! The errors of taskrock.

use thiserror::Error;

/// The errors of taskrock.
#[derive(Debug, Error)]
pub enum Error {
    /// A command that is not built yet.
    #[error("{0} is not built yet")]
    NotBuilt(&'static str),
    /// The `gh` command failed.
    #[error("gh {args} failed: {stderr}")]
    Gh { args: String, stderr: String },
    /// GitHub sent output that is not JSON.
    #[error("GitHub sent JSON that taskrock cannot read: {0}")]
    Json(String),
    /// A link taskrock does not understand.
    #[error("not a link taskrock understands: {0}")]
    Link(String),
}

impl Error {
    /// The exit code of the error.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        1
    }
}
