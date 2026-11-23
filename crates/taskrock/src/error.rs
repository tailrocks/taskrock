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
    /// A project that does not have what taskrock needs.
    #[error("{0}")]
    Project(String),
    /// The issue cannot be taken.
    #[error("{0}")]
    Refused(String),
    /// A draft file taskrock cannot read.
    #[error("{file}: {message}")]
    Draft { file: String, message: String },
}

impl Error {
    /// The exit code of the error.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Refused(_) => 3,
            _ => 1,
        }
    }
}
