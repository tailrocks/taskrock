//! Load the config of an organization from `<owner>/.github`.

use super::{Config, check, default, parse};
use crate::error::Error;
use crate::gh::Gh;

/// The config of an organization. A missing file (HTTP 404) gives
/// [`default`].
///
/// # Errors
///
/// Returns [`Error::Gh`] when GitHub fails, [`Error::Config`] when the file
/// does not parse, and [`Error::Check`] when it breaks a rule.
pub fn load(gh: &dyn Gh, owner: &str) -> Result<Config, Error> {
    match fetch(gh, owner)? {
        Some(text) => checked(&text, owner),
        None => Ok(default()),
    }
}

/// The same as [`load`], but a missing file is an error.
///
/// # Errors
///
/// Returns [`Error::Config`] when the file is missing, and the errors of
/// [`load`].
pub fn load_required(gh: &dyn Gh, owner: &str) -> Result<Config, Error> {
    match fetch(gh, owner)? {
        Some(text) => checked(&text, owner),
        None => Err(Error::Config(format!(
            "no taskrock.toml in {owner}/.github"
        ))),
    }
}

/// The text of the file, or `None` when GitHub answers HTTP 404.
fn fetch(gh: &dyn Gh, owner: &str) -> Result<Option<String>, Error> {
    let path = format!("repos/{owner}/.github/contents/taskrock.toml");
    let args = ["api", "-H", "Accept: application/vnd.github.raw", &path];
    match gh.run(&args, None) {
        Ok(text) => Ok(Some(text)),
        Err(Error::Gh { stderr, .. }) if stderr.contains("HTTP 404") => Ok(None),
        Err(error) => Err(error),
    }
}

fn checked(text: &str, owner: &str) -> Result<Config, Error> {
    let config = parse::parse(text)?;
    let problems = check::check(&config);
    if problems.is_empty() {
        return Ok(config);
    }
    for problem in &problems {
        println!("{owner}/.github/taskrock.toml: {problem}");
    }
    Err(Error::Check(problems.len()))
}
