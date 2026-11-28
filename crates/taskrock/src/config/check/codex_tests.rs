use super::check;
use crate::config::parse::parse;

const GOOD: &str = include_str!("../../../fixtures/config-good.toml");

fn with_codex(names: &[&str]) -> Vec<String> {
    let mut config = parse(GOOD).expect("the good fixture parses");
    config.review.codex = names.iter().map(|name| (*name).to_owned()).collect();
    check(&config)
}

#[test]
fn config_review_codex_accepts_bare_repository_names() {
    assert_eq!(
        with_codex(&["velnor-new", "taskrock"]),
        Vec::<String>::new()
    );
    assert_eq!(with_codex(&[]), Vec::<String>::new());
}

#[test]
fn config_review_codex_refuses_an_empty_name_and_a_slash() {
    let message = |name: &str| {
        format!("review.codex repository {name} must be a repository name of the owner, with no /")
    };
    assert_eq!(
        with_codex(&["a", "", "o/b"]),
        [message("\"\""), message("\"o/b\"")]
    );
}

#[test]
fn config_review_codex_lists_a_name_once() {
    assert_eq!(
        with_codex(&["a", "a", "a"]),
        ["review.codex repository a is listed twice"]
    );
}
