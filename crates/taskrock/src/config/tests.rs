use super::check::check;
use super::load::{load, load_required};
use super::{Visibility, default};
use crate::error::Error;
use crate::gh::FakeGh;

const GOOD: &str = include_str!("../../fixtures/config-good.toml");
const FILE: &str = "contents/taskrock.toml";
const MISSING: &str = "gh: Not Found (HTTP 404)";

#[test]
fn default_passes_check() {
    let config = default();
    assert_eq!(check(&config), Vec::<String>::new());
    assert_eq!(config.version, 1);
    assert_eq!(config.routing.visibility, Visibility::Restricted);
    assert!(config.routing.runtimes.is_empty() && config.projects.is_empty());
    assert_eq!(config.review.max_rounds, 3);
}
#[test]
fn format_finds_a_type_or_the_owner_task() {
    let config = default();
    let task = config.format("Task", false).expect("Task has a format");
    assert_eq!(task.build[2], "Steps");
    assert_eq!(config.format("Task", true), Some(&config.owner_task));
    assert_eq!(config.format("Nope", false), None);
    assert_eq!(
        config.type_names(),
        ["Initiative", "Epic", "Task", "Bug", "Feature"]
    );
}
#[test]
fn load_reads_the_file_of_the_org() {
    let gh = FakeGh::new().ok(FILE, GOOD);
    let config = load(&gh, "acme").expect("the good file loads");
    assert_eq!(config.projects.len(), 2);
    let call = &gh.calls()[0].0;
    assert!(
        call.contains("Accept: application/vnd.github.raw"),
        "{call}"
    );
    assert!(
        call.contains("repos/acme/.github/contents/taskrock.toml"),
        "{call}"
    );
}
#[test]
fn load_gives_the_default_for_a_missing_file() {
    let gh = FakeGh::new().fail(FILE, MISSING);
    assert_eq!(
        load(&gh, "acme").expect("a missing file is the default"),
        default()
    );
}
#[test]
fn load_required_refuses_a_missing_file() {
    let gh = FakeGh::new().fail(FILE, MISSING);
    let Err(Error::Config(message)) = load_required(&gh, "acme") else {
        panic!("a missing file is an error");
    };
    assert_eq!(message, "no taskrock.toml in acme/.github");
}
#[test]
fn load_counts_broken_rules_and_passes_other_failures_on() {
    let gh = FakeGh::new().ok(FILE, "version = 2\n");
    assert!(matches!(load(&gh, "acme"), Err(Error::Check(n)) if n == 6));
    let gh = FakeGh::new().fail(FILE, "gh: Server Error (HTTP 500)");
    assert!(matches!(load(&gh, "acme"), Err(Error::Gh { .. })));
}
