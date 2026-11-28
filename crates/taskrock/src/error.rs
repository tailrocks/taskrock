//! The errors of taskrock.

use thiserror::Error;

/// The errors of taskrock.
#[derive(Debug, Error)]
pub enum Error {
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
    /// A draft that breaks the check rules.
    #[error("{0} problems in the draft")]
    Check(usize),
    /// A config file taskrock cannot read or find.
    #[error("{0}")]
    Config(String),
    /// GitHub refuses to create an issue.
    #[error("{0}")]
    Apply(String),
    /// GitHub answered a `resolveReviewThread` without saying the thread is resolved.
    #[error("GitHub did not resolve the thread {0}")]
    NotResolved(String),
    /// A command that is planned and not built yet.
    #[error("taskrock {0} is not built yet")]
    NotBuilt(&'static str),
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
