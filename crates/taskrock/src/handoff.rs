//! The handoff comment of `taskrock release --note`.

use std::path::Path;

use crate::error::Error;
use crate::gh::Gh;

/// Post a file as a handoff comment. Not built yet.
///
/// # Errors
///
/// Returns [`Error::NotBuilt`].
pub fn post(_gh: &dyn Gh, _link: &str, _note: &Path) -> Result<String, Error> {
    Err(Error::NotBuilt("release --note"))
}
