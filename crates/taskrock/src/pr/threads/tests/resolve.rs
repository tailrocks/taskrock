//! `--resolve <id> --reason <text>`: a bot thread, a human thread and an unknown id.

use super::{HEAD, P1, bot, link, pull, reads, resolve, resolved, thread, writes};
use crate::error::Error;
use crate::pr::EXIT_DONE;
use crate::pr::threads::run;

fn refusal(threads: &[serde_json::Value], id: &str) -> String {
    let gh = reads(threads);
    let error = run(&gh, &link(), &resolve(id, "why"), false).expect_err("refused");
    assert_eq!(error.exit_code(), 3);
    assert_eq!(writes(&gh), Vec::<String>::new());
    assert!(gh.calls().iter().all(|(args, _)| !args.contains("-X")));
    match error {
        Error::Refused(message) => message,
        other => panic!("not a refusal: {other}"),
    }
}

#[test]
fn threads_a_blocking_thread_is_resolved_with_the_reason() {
    let gh = reads(&[bot("A", 4, P1)])
        .ok("pulls/5/comments/4/replies", "{}")
        .ok("graphql", &resolved())
        .ok("pulls/5", &pull("open", HEAD));
    let code = run(&gh, &link(), &resolve("A", "not reproducible"), false).expect("runs");
    assert_eq!(code, EXIT_DONE);
    assert_eq!(
        writes(&gh),
        [
            "reply repos/tailrocks/taskrock/pulls/5/comments/4/replies not reproducible\n\n<!-- taskrock:A -->",
            "resolve A"
        ]
    );
}

#[test]
fn threads_a_human_thread_is_refused() {
    let human = thread("H", 9, ("alice", "User"), &["please rename"], false);
    assert_eq!(refusal(&[human], "H"), "a person resolves human threads");
}

#[test]
fn threads_an_unknown_or_resolved_id_is_refused() {
    let done = thread("D", 8, ("review-bot[bot]", "Bot"), &[P1], true);
    assert_eq!(refusal(&[done], "D"), "no unresolved thread D");
    assert_eq!(refusal(&[bot("A", 1, P1)], "X"), "no unresolved thread X");
}
