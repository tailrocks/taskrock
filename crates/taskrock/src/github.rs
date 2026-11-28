//! The GitHub REST API through the `gh` command.

use crate::error::Error;
use crate::gh::Gh;

pub mod items;

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

/// POST a GraphQL query through `gh api graphql --input -` with
/// `{"query": ..., "variables": ...}`. Returns `data`.
///
/// # Errors
///
/// Returns [`Error::GraphQl`] when GitHub answered with a non-empty `errors` array
/// (the messages joined by `; `), [`Error::Gh`] when `gh` failed for any other reason
/// (no answer, an HTTP error status, bad credentials), [`Error::Json`] when the output is
/// not JSON.
pub fn graphql(
    gh: &dyn Gh,
    query: &str,
    variables: &serde_json::Value,
) -> Result<serde_json::Value, Error> {
    let input = serde_json::json!({"query": query, "variables": variables}).to_string();
    let stdout = gh
        .run(&["api", "graphql", "--input", "-"], Some(&input))
        .map_err(answered)?;
    let mut answer = parse(&stdout)?;
    if let Some(message) = error_messages(&answer) {
        return Err(Error::GraphQl { message });
    }
    Ok(answer
        .get_mut("data")
        .map_or(serde_json::Value::Null, serde_json::Value::take))
}

/// The real `gh` exits 1 when a 200 answer has `errors`: it prints the answer on standard
/// output and every message on standard error, one per line, the first after `gh: `. The
/// first line has no `(HTTP <status>)`. Any answer with an HTTP error status, and any failure
/// before an answer, is not that.
fn answered(error: Error) -> Error {
    if let Error::Gh { stderr, stdout, .. } = &error {
        let message = serde_json::from_str(stdout)
            .ok()
            .and_then(|answer| error_messages(&answer))
            .filter(|_| {
                !stderr
                    .lines()
                    .next()
                    .is_some_and(|line| line.contains("(HTTP "))
            });
        if let Some(message) = message {
            return Error::GraphQl { message };
        }
    }
    error
}

/// The messages of a non-empty `errors` array, joined by `; `.
fn error_messages(answer: &serde_json::Value) -> Option<String> {
    let errors = answer
        .get("errors")
        .and_then(serde_json::Value::as_array)
        .filter(|errors| !errors.is_empty())?;
    let messages: Vec<&str> = errors
        .iter()
        .map(|error| {
            error
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown error")
        })
        .collect();
    Some(messages.join("; "))
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
