//! `taskrock setup`: make GitHub match the organization's `taskrock.toml`.

use std::path::PathBuf;

use crate::error::Error;
use crate::gh::Gh;

/// The options of `taskrock setup`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub owner: String,
    pub file: Option<PathBuf>,
    pub dry_run: bool,
    pub yes: bool,
}

/// Make GitHub match the config. Not built yet.
///
/// # Errors
///
/// Returns [`Error::NotBuilt`].
pub fn run(_gh: &dyn Gh, _options: &Options) -> Result<String, Error> {
    Err(Error::NotBuilt("setup"))
}
