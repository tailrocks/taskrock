//! `taskrock pr merge`.

use crate::error::Error;
use crate::gh::Gh;
use crate::link::PullLink;

/// Run `taskrock pr merge`.
///
/// # Errors
///
/// Returns [`Error::NotBuilt`] until the command is built.
pub fn run(_gh: &dyn Gh, _link: &PullLink) -> Result<u8, Error> {
    Err(Error::NotBuilt("pr merge"))
}
