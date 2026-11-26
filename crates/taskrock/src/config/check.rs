//! Check a config against the rules, offline.

use std::collections::BTreeSet;
use std::path::Path;

use super::{Config, Format, Project, ProjectKind};
use crate::error::Error;

#[cfg(test)]
mod tests;

/// The issue types every config must have.
const REQUIRED_TYPES: [&str; 5] = ["Initiative", "Epic", "Task", "Bug", "Feature"];

/// Every broken rule, as messages, in file order.
#[must_use]
pub fn check(config: &Config) -> Vec<String> {
    let mut out = Vec::new();
    if config.version != 1 {
        out.push("version must be 1".to_owned());
    }
    routing(config, &mut out);
    if config.review.max_rounds < 1 {
        out.push("review.max_rounds must be at least 1".to_owned());
    }
    issue_types(config, &mut out);
    format_rules("owner task", &config.owner_task, &mut out);
    projects(config, &mut out);
    out
}

/// Read a config file, check it, return the text to print.
///
/// # Errors
///
/// Returns [`Error::Config`] when the file cannot be read, or
/// [`Error::Check`] when it breaks a rule.
pub fn run(path: &Path) -> Result<String, Error> {
    let config = super::parse::read(path)?;
    let problems = check(&config);
    if problems.is_empty() {
        return Ok(format!("OK config, {} projects", config.projects.len()));
    }
    for problem in &problems {
        println!("{}: {problem}", path.display());
    }
    Err(Error::Check(problems.len()))
}

/// Remembers the names of one kind and says the first repeat of each.
struct Names<'a> {
    kind: &'a str,
    seen: BTreeSet<&'a str>,
    said: BTreeSet<&'a str>,
}

impl<'a> Names<'a> {
    fn new(kind: &'a str) -> Self {
        Self {
            kind,
            seen: BTreeSet::new(),
            said: BTreeSet::new(),
        }
    }

    fn see(&mut self, name: &'a str) -> Option<String> {
        let repeat = !self.seen.insert(name) && self.said.insert(name);
        repeat.then(|| format!("{} {name} is listed twice", self.kind))
    }
}

fn routing(config: &Config, out: &mut Vec<String>) {
    let known = &config.routing.runtimes;
    let mut names = Names::new("subagent");
    for subagent in &config.routing.subagents {
        out.extend(names.see(&subagent.name));
        unknown_runtimes("subagent", &subagent.name, &subagent.runtimes, known, out);
        if subagent.can.is_empty() {
            out.push(format!("subagent {} has an empty can", subagent.name));
        }
    }
    let mut names = Names::new("model");
    for model in &config.routing.models {
        out.extend(names.see(&model.name));
        unknown_runtimes("model", &model.name, &model.runtimes, known, out);
        if model.efforts.is_empty() {
            out.push(format!("model {} has no efforts", model.name));
        }
    }
}

fn unknown_runtimes(
    kind: &str,
    name: &str,
    listed: &[String],
    known: &[String],
    out: &mut Vec<String>,
) {
    for runtime in listed.iter().filter(|runtime| !known.contains(runtime)) {
        out.push(format!(
            "{kind} {name} names runtime {runtime}, which is not in routing.runtimes"
        ));
    }
}

fn issue_types(config: &Config, out: &mut Vec<String>) {
    let mut names = Names::new("issue type");
    for issue_type in &config.issue_types {
        out.extend(names.see(&issue_type.name));
        let label = format!("issue type {}", issue_type.name);
        format_rules(&label, &issue_type.format, out);
    }
    for required in REQUIRED_TYPES {
        if !config.type_names().contains(&required) {
            out.push(format!("issue type {required} is missing"));
        }
    }
}

fn format_rules(label: &str, format: &Format, out: &mut Vec<String>) {
    let sets = [("file", &format.file), ("checklist", &format.checklist)];
    for (set, sections) in sets {
        for section in sections.iter().filter(|s| !format.build.contains(s)) {
            out.push(format!(
                "{label}: {set} section {section} is not a build section"
            ));
        }
    }
}

fn projects(config: &Config, out: &mut Vec<String>) {
    let mut names = Names::new("project");
    for project in &config.projects {
        out.extend(names.see(&project.key));
        project_rules(project, out);
    }
}

fn project_rules(project: &Project, out: &mut Vec<String>) {
    let key = &project.key;
    if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        out.push(format!(
            "project key {key} may have only letters, digits and -"
        ));
    }
    match project.kind {
        ProjectKind::Delivery if project.repos.is_empty() => {
            out.push(format!("project {key} is a delivery project with no repos"));
        }
        ProjectKind::Portfolio => {
            if !project.repos.is_empty() {
                out.push(format!(
                    "project {key} is a portfolio project, so it has no repos"
                ));
            }
            if !project.milestones.is_empty() {
                out.push(format!(
                    "project {key} is a portfolio project, so it has no milestones"
                ));
            }
        }
        ProjectKind::Delivery => {}
    }
    let mut titles = Names::new("milestone");
    for milestone in &project.milestones {
        out.extend(
            titles
                .see(&milestone.title)
                .map(|message| format!("project {key}: {message}")),
        );
        if let Some(due) = milestone.due.as_deref().filter(|due| !is_date(due)) {
            out.push(format!(
                "project {key}: milestone {} has due {due}, which is not a date YYYY-MM-DD",
                milestone.title
            ));
        }
    }
}

/// True when `text` is a real date `YYYY-MM-DD`.
fn is_date(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    let [year, month, day] = parts[..] else {
        return false;
    };
    let digits =
        |part: &str, len: usize| part.len() == len && part.bytes().all(|b| b.is_ascii_digit());
    if !(digits(year, 4) && digits(month, 2) && digits(day, 2)) {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        year.parse::<u32>(),
        month.parse::<u32>(),
        day.parse::<u32>(),
    ) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let last = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=last).contains(&day)
}
