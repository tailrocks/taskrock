//! `taskrock pr merge` on a real pull request of the sandbox: open, review, merge, check. A child
//! pull request on the branch of the first one ends up on `main`.

use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use taskrock::error::Error;
use taskrock::gh::{Gh as _, RealGh};
use taskrock::github::{get, send};

const REPO: &str = "repos/tailrocks/taskrock-sandbox";
const SIGN_OFF: &str = "Signed-off-by: taskrock sandbox <noreply@github.com>";
/// The author of the commit, so that the DCO check finds its sign-off matching.
const WHO: (&str, &str) = ("taskrock sandbox", "noreply@github.com");

/// Closes the pull request and deletes its branch, also when the test fails.
struct Cleanup<'a> {
    gh: &'a RealGh,
    branch: String,
    pull: Option<u64>,
}

impl Drop for Cleanup<'_> {
    fn drop(&mut self) {
        if let Some(pull) = self.pull {
            let path = format!("{REPO}/pulls/{pull}");
            match send(self.gh, "PATCH", &path, &json!({"state": "closed"})) {
                Ok(_) | Err(_) => {}
            }
        }
        let path = format!("{REPO}/git/refs/heads/{}", self.branch);
        match self.gh.run(&["api", "-X", "DELETE", &path], None) {
            Ok(_) | Err(_) => {}
        }
    }
}

fn text<'a>(value: &'a Value, what: &str) -> Result<&'a str, Error> {
    value
        .as_str()
        .ok_or_else(|| Error::Json(format!("no {what}")))
}

/// Create the branch from the branch `from` and commit one file on it. Returns the head commit.
fn push_branch(gh: &RealGh, branch: &str, from: &str, name: &str) -> Result<String, Error> {
    let source = get(gh, &format!("{REPO}/git/ref/heads/{from}"))?;
    let base = text(&source["object"]["sha"], "sha of the source branch")?;
    let ref_name = format!("refs/heads/{branch}");
    send(
        gh,
        "POST",
        &format!("{REPO}/git/refs"),
        &json!({"ref": ref_name, "sha": base}),
    )?;
    let message = format!("test: sandbox pull request {name}\n\n{SIGN_OFF}");
    let put = send(
        gh,
        "PUT",
        &format!("{REPO}/contents/sandbox/{name}.txt"),
        &json!({"message": message, "content": "c2FuZGJveAo=", "branch": branch,
            "author": {"name": WHO.0, "email": WHO.1}, "committer": {"name": WHO.0, "email": WHO.1}}),
    )?;
    Ok(text(&put["commit"]["sha"], "commit sha")?.to_owned())
}

fn open(gh: &RealGh, branch: &str, base: &str) -> Result<u64, Error> {
    let pull = send(
        gh,
        "POST",
        &format!("{REPO}/pulls"),
        &json!({"title": format!("test: {branch}"), "head": branch, "base": base,
            "body": "Live test of taskrock pr merge."}),
    )?;
    pull["number"]
        .as_u64()
        .ok_or_else(|| Error::Json("no pull number".to_owned()))
}

fn review(gh: &RealGh, number: u64, head: &str) -> Result<(), Error> {
    send(
        gh,
        "POST",
        &format!("{REPO}/pulls/{number}/reviews"),
        &json!({"event": "COMMENT", "body": format!("Reviewer: sandbox\nVERDICT: PASS head={head}")}),
    )?;
    Ok(())
}

fn taskrock(args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new(env!("CARGO_BIN_EXE_taskrock"))
        .args(args)
        .output()
}

#[test]
#[ignore = "talks to GitHub"]
fn merges_a_ready_pull_request_of_the_sandbox() {
    let gh = RealGh::default();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_secs();
    let branch = format!("sandbox/pr-{stamp}");
    let child_branch = format!("sandbox/child-{stamp}");
    let mut cleanup = Cleanup {
        gh: &gh,
        branch: branch.clone(),
        pull: None,
    };
    let mut child_cleanup = Cleanup {
        gh: &gh,
        branch: child_branch.clone(),
        pull: None,
    };
    let head = push_branch(&gh, &branch, "main", &format!("pr-{stamp}")).expect("branch pushed");
    let number = open(&gh, &branch, "main").expect("pull request opened");
    cleanup.pull = Some(number);
    review(&gh, number, &head).expect("verdict posted");
    push_branch(&gh, &child_branch, &branch, &format!("child-{stamp}")).expect("child pushed");
    let child = open(&gh, &child_branch, &branch).expect("child opened");
    child_cleanup.pull = Some(child);
    let link = format!("https://github.com/tailrocks/taskrock-sandbox/pull/{number}");

    let deadline = Instant::now() + Duration::from_secs(280);
    let (code, said) = loop {
        let output = taskrock(&["pr", "merge", &link]).expect("the taskrock binary starts");
        let code = output.status.code();
        if code != Some(10) || Instant::now() > deadline {
            let said = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            break (code, said);
        }
        std::thread::sleep(Duration::from_secs(5));
    };
    assert_eq!(code, Some(0), "pr merge ends with 0: {said}");

    let status = taskrock(&["pr", "status", &link]).expect("the taskrock binary starts");
    let out = String::from_utf8(status.stdout).expect("stdout is UTF-8");
    assert!(out.contains("MERGED"), "{out}");

    // Whether GitHub deleted the head branch (and retargeted the child itself) or `pr merge` did.
    let pull = get(&gh, &format!("{REPO}/pulls/{child}")).expect("the child is readable");
    assert_eq!(
        pull["base"]["ref"], "main",
        "the child points at main: {said}"
    );
    assert_eq!(pull["state"], "open", "the child stays open");
}
