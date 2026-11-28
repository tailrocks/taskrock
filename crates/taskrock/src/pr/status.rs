//! `taskrock pr status`.

use crate::error::Error;
use crate::gh::Gh;
use crate::link::PullLink;

pub mod code;
pub mod evaluate;

/// Run `taskrock pr status`.
///
/// # Errors
///
/// Returns [`Error::NotBuilt`] until the command is built.
pub fn run(_gh: &dyn Gh, _link: &PullLink) -> Result<u8, Error> {
    Err(Error::NotBuilt("pr status"))
}
