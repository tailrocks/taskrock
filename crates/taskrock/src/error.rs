//! The errors of taskrock.

use thiserror::Error;

/// The errors of taskrock.
#[derive(Debug, Error)]
pub enum Error {
    /// A command that is not built yet.
    #[error("{0} is not built yet")]
    NotBuilt(&'static str),
}

impl Error {
    /// The exit code of the error.
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        1
    }
}
