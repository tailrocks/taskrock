use super::{parse, read};
use crate::config::{Ability, ProjectKind, Visibility, default};
use crate::error::Error;
use std::path::Path;

const GOOD: &str = include_str!("../../../fixtures/config-good.toml");

#[test]
fn the_good_fixture_parses_into_every_part() {
    let config = parse(GOOD).expect("the good fixture parses");
    assert_eq!(config.version, 1);
    assert_eq!(
        config.routing.runtimes,
        ["claude", "codex", "agy", "muse", "grok"]
    );
    assert_eq!(config.routing.visibility, Visibility::Restricted);
    assert_eq!(config.routing.subagents[1].can, [Ability::Review]);
    assert_eq!(config.routing.models[0].name, "claude-haiku-5-5");
    assert_eq!(config.review.max_rounds, 3);
    assert_eq!(
        config.type_names(),
        ["Initiative", "Epic", "Task", "Bug", "Feature"]
    );
    assert_eq!(config.owner_task, default().owner_task);
    assert_eq!(config.projects[0].number, Some(3));
    assert_eq!(
        config.projects[0].milestones[0].due.as_deref(),
        Some("2026-12-15")
    );
    assert_eq!(config.projects[1].kind, ProjectKind::Portfolio);
}
#[test]
fn a_small_file_gets_the_defaults() {
    let config = parse("version = 1\n").expect("the file parses");
    assert_eq!(config.review.max_rounds, 3);
    assert_eq!(config.owner_task, default().owner_task);
    assert!(config.issue_types.is_empty() && config.projects.is_empty());
    let project = "[[project]]\nkey = \"a\"\ntitle = \"A\"\nkind = \"delivery\"\n";
    let config = parse(&format!("version = 1\n{project}")).expect("the file parses");
    assert!(config.projects[0].workflows.pr_merged);
}
#[test]
fn an_unknown_key_is_refused() {
    for text in [
        "version = 1\nextra = 2\n",
        "version = 1\n[routing]\nextra = 2\n",
        "version = 1\n[review]\nmax_rounds = 2\nextra = 2\n",
        "version = 1\n[[project]]\nkey = \"a\"\ntitle = \"A\"\nkind = \"delivery\"\nextra = 1\n",
        "version = 1\n[owner_task]\nfile = []\nbuild = []\nextra = 1\n",
    ] {
        let Err(Error::Config(message)) = parse(text) else {
            panic!("{text} should be refused");
        };
        assert!(message.contains("unknown field"), "{message}");
    }
}
#[test]
fn a_wrong_value_is_refused() {
    assert!(matches!(
        parse("version = 1\n[routing]\nvisibility = \"some\"\n"),
        Err(Error::Config(_))
    ));
}
#[test]
fn read_names_the_path() {
    let Err(Error::Config(message)) = read(Path::new("/no/such/taskrock.toml")) else {
        panic!("a missing file is an error");
    };
    assert!(message.starts_with("/no/such/taskrock.toml: "), "{message}");
}
