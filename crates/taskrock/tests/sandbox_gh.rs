//! The sandbox repository on GitHub.

use taskrock::gh::RealGh;
use taskrock::github::get;

#[test]
#[ignore = "talks to GitHub"]
fn reads_the_sandbox_repository() {
    let repo = get(&RealGh::default(), "repos/tailrocks/taskrock-sandbox").expect("GitHub answers");
    assert_eq!(repo["name"], "taskrock-sandbox");
}
