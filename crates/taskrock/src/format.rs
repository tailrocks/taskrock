//! The sections of an issue body, checked against a format of the config.

#[cfg(test)]
mod tests;

/// The text of section `name`, trimmed, or None when the heading is missing.
///
/// A section is the lines after a line whose trimmed text is `## <name>`, up
/// to the next line that starts with `## `. Lines inside fenced code blocks
/// never start or end a section.
#[must_use]
pub fn section(body: &str, name: &str) -> Option<String> {
    let heading = format!("## {name}");
    let mut fenced = false;
    let mut found = false;
    let mut lines: Vec<&str> = Vec::new();
    for line in body.lines() {
        let fence = line.starts_with("```");
        if found {
            if !fenced && line.starts_with("## ") {
                break;
            }
            lines.push(line);
        } else if !fenced && line.trim() == heading {
            found = true;
        }
        if fence {
            fenced = !fenced;
        }
    }
    found.then(|| lines.join("\n").trim().to_owned())
}

/// One message per broken rule of `format` on `body`, using `sections` as the
/// wanted list.
#[must_use]
pub fn problems(body: &str, sections: &[String], checklist: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for name in sections {
        match section(body, name) {
            None => out.push(format!("the section ## {name} is missing")),
            Some(text) if text.is_empty() => out.push(format!(
                "the section ## {name} is empty. Write None. and the reason"
            )),
            Some(_) => {}
        }
    }
    for name in checklist {
        if let Some(text) = section(body, name) {
            out.extend(box_problems(name, &text));
        }
    }
    out
}

fn box_problems(name: &str, text: &str) -> Vec<String> {
    let boxes: Vec<&str> = text.lines().map(str::trim).filter(|l| is_box(l)).collect();
    if boxes.is_empty() {
        return vec![format!("## {name} has no checkbox")];
    }
    let mut out = Vec::new();
    for (index, line) in boxes.iter().enumerate() {
        let number = index + 1;
        match end_of_first_span(line) {
            None => out.push(format!(
                "## {name}: box {number} has no command in backticks"
            )),
            Some(end) if !has_result(&line[end..]) => out.push(format!(
                "## {name}: box {number} has no result after the command"
            )),
            Some(_) => {}
        }
    }
    out
}

fn is_box(line: &str) -> bool {
    line.starts_with("- [ ]") || line.starts_with("- [x]")
}

/// The byte index just after the first code span of `line`, if it has one.
fn end_of_first_span(line: &str) -> Option<usize> {
    let mut ticks = line.match_indices('`').map(|(index, _)| index);
    ticks.next()?;
    ticks.next().map(|index| index + 1)
}

/// True when `text` has a letter or a digit outside any code span.
fn has_result(text: &str) -> bool {
    outside_code(text).any(char::is_alphanumeric)
}

/// The characters of `text` that are not inside a code span, with a space
/// where a span was. An unpaired backtick opens no span.
fn outside_code(text: &str) -> impl Iterator<Item = char> + '_ {
    let ticks: Vec<usize> = text.match_indices('`').map(|(index, _)| index).collect();
    let paired = ticks.len() - ticks.len() % 2;
    text.char_indices().filter_map(move |(index, ch)| {
        let inside = ticks[..paired]
            .chunks(2)
            .any(|pair| index >= pair[0] && index <= pair[1]);
        (!inside).then_some(ch)
    })
}

/// One message per bare issue number outside code.
#[must_use]
pub fn bare_numbers(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut fenced = false;
    for line in body.lines() {
        if line.starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if !fenced {
            out.extend(line_numbers(line));
        }
    }
    out
}

fn line_numbers(line: &str) -> Vec<String> {
    let ticks: Vec<usize> = line.match_indices('`').map(|(index, _)| index).collect();
    let paired = ticks.len() - ticks.len() % 2;
    let in_span = |at: usize| {
        ticks[..paired]
            .chunks(2)
            .any(|pair| at >= pair[0] && at <= pair[1])
    };
    let mut out = Vec::new();
    let mut before: Option<char> = None;
    for (index, ch) in line.char_indices() {
        let bare_start = ch == '#'
            && !in_span(index)
            && before.is_none_or(|b| !(b.is_alphanumeric() || b == '&' || b == '/'));
        if bare_start {
            let digits: String = line[index + 1..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            if !digits.is_empty() {
                out.push(format!(
                    "#{digits} is a bare issue number. Name the key instead"
                ));
            }
        }
        before = Some(ch);
    }
    out
}
