//! The GitHub REST API through the `gh` command.

use crate::error::Error;
use crate::gh::Gh;

#[cfg(test)]
mod tests;

/// `gh api <path>`. Returns the parsed JSON.
///
/// # Errors
///
/// Returns [`Error::Gh`] when `gh` fails, [`Error::Json`] when the output is not JSON.
pub fn get(gh: &dyn Gh, path: &str) -> Result<serde_json::Value, Error> {
    let stdout = gh.run(&["api", path], None)?;
    parse(&stdout)
}

/// `gh api --paginate --slurp <path>`. GitHub returns one array per page.
/// This returns the items of all pages as one list.
///
/// # Errors
///
/// Returns [`Error::Gh`] when `gh` fails, [`Error::Json`] when the output is not JSON.
pub fn get_all(gh: &dyn Gh, path: &str) -> Result<Vec<serde_json::Value>, Error> {
    let stdout = gh.run(&["api", "--paginate", "--slurp", path], None)?;
    let pages: Vec<Vec<serde_json::Value>> =
        serde_json::from_str(&stdout).map_err(|error| Error::Json(error.to_string()))?;
    Ok(pages.into_iter().flatten().collect())
}

/// `gh api -X <method> <path> --input -` with `body` as the input.
/// Returns the parsed JSON, or `Value::Null` when the output is empty.
///
/// # Errors
///
/// Returns [`Error::Gh`] when `gh` fails, [`Error::Json`] when the output is not JSON.
pub fn send(
    gh: &dyn Gh,
    method: &str,
    path: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, Error> {
    let input = body.to_string();
    let stdout = gh.run(&["api", "-X", method, path, "--input", "-"], Some(&input))?;
    if stdout.trim().is_empty() {
        Ok(serde_json::Value::Null)
    } else {
        parse(&stdout)
    }
}

/// True when `error` is `Error::Gh` and its text contains `(HTTP <code>)`.
#[must_use]
pub fn is_status(error: &Error, code: u16) -> bool {
    match error {
        Error::Gh { .. } => error.to_string().contains(&format!("(HTTP {code})")),
        _ => false,
    }
}

fn parse(stdout: &str) -> Result<serde_json::Value, Error> {
    serde_json::from_str(stdout).map_err(|error| Error::Json(error.to_string()))
}
