//! `taskrock template`: print an empty body with the file sections of a type.

use std::path::Path;

use crate::error::Error;
use crate::gh::Gh;

/// Print an empty body for a type. Not built yet.
///
/// # Errors
///
/// Returns [`Error::NotBuilt`].
pub fn run(
    _gh: &dyn Gh,
    _kind: &str,
    _owner: Option<&str>,
    _config: Option<&Path>,
) -> Result<String, Error> {
    Err(Error::NotBuilt("template"))
}
