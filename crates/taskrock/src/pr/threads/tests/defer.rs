//! `--defer`: two threads, a rerun, a moved head and a failed resolve.

use super::{HEAD, P1, P3, bot, link, reads, resolved, thread, writes};
use crate::error::Error;
use crate::pr::threads::{Action, run};
use crate::pr::{EXIT_BLOCKED, EXIT_DONE};

const REPLY: &str = "Deferred: severity P3 is low, so it does not block this pull request.";

#[test]
fn threads_defer_replies_to_and_resolves_each_deferrable_thread() {
    let human = thread("H", 9, ("alice", "User"), &[P3], false);
    let done = thread("D", 8, ("review-bot[bot]", "Bot"), &[P3], true);
    let threads = vec![
        bot("A", 1, P3),
        bot("B", 2, P1),
        bot("C", 3, P3),
        human,
        done,
    ];
    let gh = reads(threads)
        .ok("pulls/5/comments/1/replies", "{}")
        .ok("pulls/5/comments/3/replies", "{}")
        .ok("graphql", &resolved())
        .ok("graphql", &resolved())
        .ok("pulls/5", &super::pull("open", HEAD));
    assert_eq!(
        run(&gh, &link(), &Action::Defer, false).expect("runs"),
        EXIT_DONE
    );
    let repo = "repos/tailrocks/taskrock";
    assert_eq!(
        writes(&gh),
        [
            format!("reply {repo}/pulls/5/comments/1/replies {REPLY}\n\n<!-- taskrock:A -->"),
            "resolve A".to_owned(),
            format!("reply {repo}/pulls/5/comments/3/replies {REPLY}\n\n<!-- taskrock:C -->"),
            "resolve C".to_owned(),
        ]
    );
}

#[test]
fn threads_rerun_with_the_marker_only_resolves() {
    let replied = thread(
        "A",
        1,
        ("review-bot[bot]", "Bot"),
        &[P3, &format!("{REPLY}\n\n<!-- taskrock:A -->")],
        false,
    );
    let gh = reads(vec![replied])
        .ok("graphql", &resolved())
        .ok("pulls/5", &super::pull("open", HEAD));
    assert_eq!(
        run(&gh, &link(), &Action::Defer, false).expect("runs"),
        EXIT_DONE
    );
    assert_eq!(writes(&gh), ["resolve A"]);
}

#[test]
fn threads_a_moved_head_or_a_closed_pull_refuses_before_any_write() {
    for fresh in [
        super::pull("open", &"f".repeat(40)),
        super::pull("closed", HEAD),
    ] {
        let gh = reads(vec![bot("A", 1, P3)]).ok("pulls/5", &fresh);
        let error = run(&gh, &link(), &Action::Defer, false).expect_err("refused");
        assert!(
            matches!(&error, Error::Refused(m) if m == "the pull request changed; run the command again")
        );
        assert_eq!(error.exit_code(), 3);
        assert!(writes(&gh).is_empty());
    }
}

#[test]
fn threads_a_failed_resolve_stops_the_run() {
    let gh = reads(vec![bot("A", 1, P3), bot("B", 2, P3)])
        .ok("pulls/5/comments/1/replies", "{}")
        .fail("graphql", "gh: Could not resolve")
        .ok("pulls/5", &super::pull("open", HEAD));
    assert_eq!(
        run(&gh, &link(), &Action::Defer, false).expect("runs"),
        EXIT_BLOCKED
    );
    let made = writes(&gh);
    assert_eq!(made.len(), 2, "{made:?}");
    assert!(made[0].starts_with("reply ") && made[1] == "resolve A");
}
