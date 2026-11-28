//! The severity that a review bot writes at the top of a comment.

use std::fmt;

#[cfg(test)]
mod tests;

/// How serious a finding is: 0 is the worst.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Severity(pub u8);

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "P{}", self.0)
    }
}

/// The lowest severity that a badge names in the first five non-empty lines
/// of `body`: `![P<n> Badge](` or `badge/P<n>-`.
#[must_use]
pub fn parse(body: &str) -> Option<Severity> {
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(5)
        .flat_map(|line| badges(line, "![P", " Badge](").chain(badges(line, "badge/P", "-")))
        .min()
}

/// The severities in `line` written as `<open><digits><close>`.
fn badges<'a>(line: &'a str, open: &'a str, close: &'a str) -> impl Iterator<Item = Severity> + 'a {
    line.match_indices(open).filter_map(move |(at, _)| {
        let rest = &line[at + open.len()..];
        let (digits, after) = rest.split_at(rest.bytes().take_while(u8::is_ascii_digit).count());
        after.starts_with(close).then_some(())?;
        digits.parse().ok().map(Severity)
    })
}
