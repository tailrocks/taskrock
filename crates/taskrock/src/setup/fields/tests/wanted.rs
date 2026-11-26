//! Tests of the wanted options.

use super::{good, names};
use crate::config::Model;
use crate::setup::fields::wanted;

#[test]
fn wanted_lists_the_options_of_the_eight_fields() {
    let list = wanted(&good());
    let fields: Vec<&str> = list.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        fields,
        [
            "Agent",
            "Review agent",
            "Runtime",
            "Review runtime",
            "Model",
            "Review model",
            "Effort",
            "Review effort"
        ]
    );
    assert_eq!(names(&list[0].1), ["Owner", "tailrocks-systems-engineer"]);
    assert_eq!(
        names(&list[1].1),
        ["tailrocks-systems-engineer", "tailrocks-code-reviewer"]
    );
    assert_eq!(
        names(&list[3].1),
        ["claude", "codex", "agy", "muse", "grok"]
    );
    assert_eq!(
        names(&list[5].1),
        ["claude-haiku-5-5", "claude-sonnet-5-5", "claude-opus-5-5"]
    );
    assert_eq!(names(&list[7].1), ["low", "medium", "high", "xhigh", "max"]);
}

#[test]
fn wanted_efforts_keep_first_appearance_without_repeats() {
    let mut config = good();
    config.routing.models.push(Model {
        name: "big".to_owned(),
        runtimes: vec!["claude".to_owned()],
        efforts: vec!["high".to_owned(), "ultra".to_owned()],
        usage: String::new(),
    });
    let list = wanted(&config);
    assert_eq!(
        names(&list[6].1),
        ["low", "medium", "high", "xhigh", "max", "ultra"]
    );
    assert_eq!(names(&list[4].1)[3], "big");
}
