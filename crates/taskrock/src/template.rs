//! `taskrock template`: print an empty body with the file sections of a type.

use std::path::Path;

use crate::config::{self, Config, Format};
use crate::error::Error;
use crate::gh::Gh;

#[cfg(test)]
mod tests;

/// Print an empty body for a type: the file sections of its format, each as
/// a `## <name>` heading. A checklist section also has an empty box.
///
/// The config is read from `config` when it is given, else from
/// `<owner>/.github` when `owner` is given, else it is the built-in default.
/// `kind` is a type name, or `owner` for the owner task.
///
/// # Errors
///
/// Returns [`Error::Config`] when the config cannot be read or `kind` is
/// unknown, and [`Error::Gh`] or [`Error::Check`] as [`config::load`] does.
pub fn run(
    gh: &dyn Gh,
    kind: &str,
    owner: Option<&str>,
    config: Option<&Path>,
) -> Result<String, Error> {
    let loaded = match (config, owner) {
        (Some(path), _) => config::parse::read(path)?,
        (None, Some(owner)) => config::load::load(gh, owner)?,
        (None, None) => config::default(),
    };
    let format = find(&loaded, kind)?;
    Ok(render(format, &format.checklist))
}

fn find<'a>(config: &'a Config, kind: &str) -> Result<&'a Format, Error> {
    config.format(kind, kind == "owner").ok_or_else(|| {
        Error::Config(format!(
            "unknown type {kind}. Types: {}, owner",
            config.type_names().join(", ")
        ))
    })
}

fn render(format: &Format, checklist: &[String]) -> String {
    let sections: Vec<String> = format
        .file
        .iter()
        .map(|name| {
            let mut lines = vec![format!("## {name}"), String::new()];
            if checklist.contains(name) {
                lines.push("- [ ] `<command>` <result>".to_owned());
            }
            lines.join("\n")
        })
        .collect();
    sections.join("\n\n").trim_end().to_owned()
}
