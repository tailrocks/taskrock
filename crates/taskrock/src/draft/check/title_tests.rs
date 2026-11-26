use super::tests::{issue, lines};

const MESSAGE: &str = "T.md: the title starts with a lowercase letter. Start with a capital letter or a digit, put the code name in backticks, or start with the name of the repository";

fn titled(title: &str) -> Vec<String> {
    let mut task = issue("T", "");
    task.title = title.to_owned();
    lines(vec![task])
}

#[test]
fn a_lowercase_title_is_reported() {
    assert_eq!(titled("add CSV export"), [MESSAGE]);
    assert_eq!(titled("tailrocks 0.2.0: CSV"), [MESSAGE]);
}

#[test]
fn capitals_digits_code_and_brackets_are_allowed() {
    for good in [
        "Add CSV",
        "2 ways to export",
        "`claim` refuses a draft",
        "[apply test] epic",
    ] {
        assert_eq!(titled(good), Vec::<String>::new(), "{good}");
    }
}

#[test]
fn the_repository_name_may_start_a_title() {
    assert_eq!(titled("holla 0.2.0: CSV"), Vec::<String>::new());
    assert_eq!(titled("holla: notes"), Vec::<String>::new());
    assert_eq!(titled("hollax 0.2.0"), [MESSAGE]);
}
