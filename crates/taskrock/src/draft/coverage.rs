//! `taskrock check --coverage`: a read-only coverage report of a draft.

use std::path::Path;

use crate::config::Config;
use crate::error::Error;

/// Print the coverage report of a draft. Not built yet.
///
/// # Errors
///
/// Returns [`Error::NotBuilt`].
pub fn run(_folder: &Path, _config: &Config) -> Result<String, Error> {
    Err(Error::NotBuilt("check --coverage"))
}
