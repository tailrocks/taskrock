//! The handoff comment of `taskrock release --note`.

use std::path::Path;

use crate::error::Error;
use crate::format;
use crate::gh::Gh;
use crate::github::send;
use crate::link::IssueLink;

#[cfg(test)]
mod tests;

const AGENT_LINE: &str = "_Written by an agent._";
const SECTIONS: [&str; 4] = ["Done", "Verified", "State", "Next step"];

/// Post a note file as a handoff comment on the issue at `link`.
///
/// The note is a Markdown file with the sections `Done`, `Verified`, `State`
/// and `Next step`. The comment is `_Written by an agent._`, an empty line,
/// then the note text.
///
/// Returns `note posted <html_url of the comment>`.
///
/// # Errors
///
/// Returns [`Error::Link`] when `link` is not an issue link,
/// [`Error::Draft`] when the file cannot be read or breaks a rule of the
/// format (before any network call), and [`Error::Gh`] or [`Error::Json`]
/// when GitHub fails or answers badly.
pub fn post(gh: &dyn Gh, link: &str, note: &Path) -> Result<String, Error> {
    let issue = link.parse::<IssueLink>()?;
    let file = note.display().to_string();
    let text = std::fs::read_to_string(note).map_err(|error| Error::Draft {
        file: file.clone(),
        message: error.to_string(),
    })?;
    let sections: Vec<String> = SECTIONS.iter().map(|name| (*name).to_owned()).collect();
    let messages = format::problems(&text, &sections, &[]);
    if !messages.is_empty() {
        return Err(Error::Draft {
            file,
            message: messages.join("; "),
        });
    }
    let body = format!("{AGENT_LINE}\n\n{text}");
    let path = format!("{}/comments", issue.api_path());
    let answer = send(gh, "POST", &path, &serde_json::json!({ "body": body }))?;
    let url = answer
        .get("html_url")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| Error::Json("the comment has no html_url".to_owned()))?;
    Ok(format!("note posted {url}"))
}
