//! A pull request snapshot from GitHub.

use taskrock::gh::RealGh;
use taskrock::link::PullLink;
use taskrock::pr::snapshot::{PullState, read};

#[test]
#[ignore = "talks to GitHub"]
fn reads_a_merged_pull_request_of_velnor_new() {
    let link: PullLink = "https://github.com/tailrocks/velnor-new/pull/2932"
        .parse()
        .expect("a pull request link");
    let snapshot = read(&RealGh::default(), &link).expect("GitHub answers");
    assert_eq!(snapshot.number, 2932);
    assert_eq!(snapshot.state, PullState::Merged);
    assert!(snapshot.merge_commit.is_some());
    assert_eq!(snapshot.base_ref, "main");
    assert_eq!(snapshot.checks.len(), 18);
    assert_eq!(snapshot.threads.len(), 76);
    assert!(snapshot.queue.has_queue);
    assert!(snapshot.codex.is_some());
    assert!(snapshot.problems.is_empty(), "{:?}", snapshot.problems);
}
