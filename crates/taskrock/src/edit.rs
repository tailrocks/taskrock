//! `taskrock edit`: replace the body of an issue after checking it.

use std::path::Path;

use crate::error::Error;
use crate::gh::Gh;

/// Replace the body of an issue. Not built yet.
///
/// # Errors
///
/// Returns [`Error::NotBuilt`].
pub fn run(_gh: &dyn Gh, _link: &str, _file: &Path, _replan: bool) -> Result<String, Error> {
    Err(Error::NotBuilt("edit"))
}
