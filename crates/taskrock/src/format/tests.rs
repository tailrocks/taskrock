use super::{bare_numbers, problems, section};

fn names(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

fn done(body: &str) -> Vec<String> {
    problems(
        &format!("## Done when\n\n{body}\n"),
        &names(&["Done when"]),
        &names(&["Done when"]),
    )
}

#[test]
fn section_is_the_trimmed_text_up_to_the_next_heading() {
    let body = "## A\n\n one \n\n### sub\n\n## B\ntwo\n";
    assert_eq!(section(body, "A").as_deref(), Some("one \n\n### sub"));
    assert_eq!(section(body, "B").as_deref(), Some("two"));
    assert_eq!(section(body, "C"), None);
}

#[test]
fn a_heading_inside_a_fenced_block_starts_and_ends_nothing() {
    let body = "## A\n\n```\n## B\n```\n\nafter\n\n## C\nc\n";
    assert_eq!(section(body, "B"), None);
    assert_eq!(
        section(body, "A").as_deref(),
        Some("```\n## B\n```\n\nafter")
    );
}

#[test]
fn a_missing_section_is_reported() {
    let found = problems("## A\nx\n", &names(&["A", "B"]), &[]);
    assert_eq!(found, ["the section ## B is missing"]);
}

#[test]
fn an_empty_section_is_reported() {
    let found = problems("## A\n\n## B\nb\n", &names(&["A", "B"]), &[]);
    assert_eq!(
        found,
        ["the section ## A is empty. Write None. and the reason"]
    );
}

#[test]
fn a_checklist_without_a_box_is_reported() {
    assert_eq!(done("It works."), ["## Done when has no checkbox"]);
}

#[test]
fn a_box_without_a_command_is_reported() {
    let found = done("- [ ] `a` prints ok.\n- [x] Tests pass.");
    assert_eq!(found, ["## Done when: box 2 has no command in backticks"]);
}

#[test]
fn a_box_without_a_result_is_reported() {
    let found = done("- [ ] `a`\n- [ ] `a` prints `ok`.\n- [ ] `a` `b`.\n- [ ] `a` exits 0.");
    assert_eq!(
        found,
        [
            "## Done when: box 1 has no result after the command",
            "## Done when: box 3 has no result after the command",
        ]
    );
}

#[test]
fn a_checklist_that_is_absent_is_only_reported_missing() {
    let found = problems("", &names(&["Done when"]), &names(&["Done when"]));
    assert_eq!(found, ["the section ## Done when is missing"]);
}

#[test]
fn a_bare_number_is_reported() {
    let found = bare_numbers("Fixes #12 and (#3).\n#7");
    let expected = "is a bare issue number. Name the key instead";
    assert_eq!(
        found,
        [
            format!("#12 {expected}"),
            format!("#3 {expected}"),
            format!("#7 {expected}")
        ]
    );
}

#[test]
fn a_number_in_code_or_after_a_letter_or_ampersand_is_not_reported() {
    assert_eq!(bare_numbers("See `#12` here."), Vec::<String>::new());
    assert_eq!(bare_numbers("```\n#12\n```"), Vec::<String>::new());
    assert_eq!(
        bare_numbers("&#12; and a#1 and tailrocks/taskrock#5"),
        Vec::<String>::new()
    );
    assert_eq!(bare_numbers("A # sign and #tag."), Vec::<String>::new());
}
