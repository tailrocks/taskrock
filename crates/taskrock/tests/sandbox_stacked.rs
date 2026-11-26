//! Stacked claim against GitHub: a blocked task waits, then starts from its blocker's branch.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::json;

use taskrock::claim::{claim, claim_stacked};
use taskrock::error::Error;
use taskrock::gh::{Gh as _, RealGh};
use taskrock::github::items::{Item, project_items};
use taskrock::github::{get, get_all, send};
use taskrock::link::ProjectLink;
use taskrock::ready;

const REPO: &str = "repos/tailrocks/taskrock-sandbox";
const BODY: &str = "## Outcome\n\nx\n\n## Decisions already made\n\nx\n\n## Steps\n\nx\n\n## Done when\n\n- [ ] `true` exits with 0\n\n## Out of scope\n\nx\n";

fn project() -> ProjectLink {
    ProjectLink {
        owner: "tailrocks".to_owned(),
        number: 10,
    }
}

fn number(value: &serde_json::Value) -> Result<u64, Error> {
    value["number"]
        .as_u64()
        .ok_or_else(|| Error::Json("number".to_owned()))
}

fn wait_until(what: &str, mut ready: impl FnMut() -> Result<bool, Error>) -> Result<(), Error> {
    let deadline = Instant::now() + Duration::from_secs(280);
    loop {
        if ready()? {
            return Ok(());
        }
        if Instant::now() > deadline {
            return Err(Error::Project(what.to_owned()));
        }
        std::thread::sleep(Duration::from_secs(5));
    }
}

fn add_and_wait(gh: &RealGh, issues: &[u64]) -> Result<(), Error> {
    send(
        gh,
        "POST",
        "orgs/tailrocks/projectsV2/10/items",
        &json!({"type": "Issue", "owner": "tailrocks", "repo": "taskrock-sandbox", "number": issues[0]}),
    )?;
    wait_until("the issues join project 10", || {
        let items = project_items(gh, &project())?;
        Ok(issues
            .iter()
            .all(|issue| items.iter().any(|item| item.link.number == *issue)))
    })
}

fn wait_for_blocker(gh: &RealGh, stacked: u64) -> Result<(), Error> {
    wait_until("the blocker link shows in project 10", || {
        let items = project_items(gh, &project())?;
        Ok(items
            .iter()
            .find(|item: &&Item| item.link.number == stacked)
            .is_some_and(|item| item.open_blockers == 1))
    })
}

fn create_issues(gh: &RealGh, stamp: u64) -> Result<(u64, u64, u64), Error> {
    let epic = send(
        gh,
        "POST",
        &format!("{REPO}/issues"),
        &json!({"title": format!("[stacked test] epic {stamp}"), "type": "Epic"}),
    )?;
    let blocker = send(
        gh,
        "POST",
        &format!("{REPO}/issues"),
        &json!({"title": format!("[stacked test] blocker {stamp}"), "type": "Task", "body": BODY, "parent_issue_id": epic["id"]}),
    )?;
    let stacked = send(
        gh,
        "POST",
        &format!("{REPO}/issues"),
        &json!({"title": format!("[stacked test] stacked {stamp}"), "type": "Task", "body": BODY, "parent_issue_id": epic["id"]}),
    )?;
    send(
        gh,
        "POST",
        &format!(
            "{REPO}/issues/{}/dependencies/blocked_by",
            number(&stacked)?
        ),
        &json!({"issue_id": blocker["id"]}),
    )?;
    Ok((number(&epic)?, number(&blocker)?, number(&stacked)?))
}

/// Commits a file on the blocker branch and opens its pull request.
fn commit_and_open_pr(gh: &RealGh, blocker: u64, stamp: u64) -> Result<u64, Error> {
    send(
        gh,
        "PUT",
        &format!("{REPO}/contents/stacked-test/{stamp}.txt"),
        &json!({"message": "test: stacked base", "content": "c3RhY2tlZAo=", "branch": format!("feat/issue-{blocker}")}),
    )?;
    let pr = send(
        gh,
        "POST",
        &format!("{REPO}/pulls"),
        &json!({"title": format!("[stacked test] {stamp}"), "head": format!("feat/issue-{blocker}"), "base": "main", "body": "Live test of taskrock claim --stacked. Closed by the test."}),
    )?;
    number(&pr)
}

fn blocker_sha(gh: &RealGh, blocker: u64) -> Result<String, Error> {
    let answer = get(gh, &format!("{REPO}/git/ref/heads/feat/issue-{blocker}"))?;
    answer["object"]["sha"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| Error::Json("sha".to_owned()))
}

fn delete_branch(gh: &RealGh, issue: u64) {
    let path = format!("{REPO}/git/refs/heads/feat/issue-{issue}");
    match gh.run(&["api", "-X", "DELETE", &path], None) {
        Ok(_) | Err(_) => {}
    }
}

fn cleanup(
    gh: &RealGh,
    epic: u64,
    blocker: u64,
    stacked: u64,
    pr: Option<u64>,
) -> Result<(), Error> {
    if let Some(pr) = pr {
        send(
            gh,
            "PATCH",
            &format!("{REPO}/pulls/{pr}"),
            &json!({"state": "closed"}),
        )?;
    }
    delete_branch(gh, blocker);
    delete_branch(gh, stacked);
    for issue in [epic, blocker, stacked] {
        send(
            gh,
            "PATCH",
            &format!("{REPO}/issues/{issue}"),
            &json!({"state": "closed"}),
        )?;
    }
    Ok(())
}

fn ready_entry(out: &str, number: u64) -> Result<Option<serde_json::Value>, Error> {
    let value: serde_json::Value =
        serde_json::from_str(out).map_err(|error| Error::Json(error.to_string()))?;
    let entries = value["ready"]
        .as_array()
        .ok_or_else(|| Error::Json("ready".to_owned()))?;
    Ok(entries
        .iter()
        .find(|entry| entry["number"].as_u64() == Some(number))
        .cloned())
}

#[test]
#[ignore = "talks to GitHub"]
fn stacked_claim_starts_from_the_blocker_branch() {
    let gh = RealGh::default();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_secs();
    let (epic, blocker, stacked) = create_issues(&gh, stamp).expect("issues created");
    let epic_url = format!("https://github.com/tailrocks/taskrock-sandbox/issues/{epic}");
    let blocker_url = format!("https://github.com/tailrocks/taskrock-sandbox/issues/{blocker}");
    let stacked_url = format!("https://github.com/tailrocks/taskrock-sandbox/issues/{stacked}");
    let setup = add_and_wait(&gh, &[epic, blocker, stacked])
        .and_then(|()| wait_for_blocker(&gh, stacked))
        .and_then(|()| claim(&gh, &blocker_url).map(|_| ()))
        .and_then(|()| commit_and_open_pr(&gh, blocker, stamp).map(Some))
        .and_then(|pr| blocker_sha(&gh, blocker).map(|sha| (pr, sha)));
    let (pr, sha) = match setup {
        Ok((pr, sha)) => (pr, sha),
        Err(error) => {
            cleanup(&gh, epic, blocker, stacked, None).expect("issues closed");
            panic!("setup failed: {error}");
        }
    };
    let refused = claim(&gh, &stacked_url);
    let ready_stacked = ready::run(&gh, &epic_url, true, true);
    let ready_plain = ready::run(&gh, &epic_url, true, false);
    let claimed = claim_stacked(&gh, &stacked_url);
    let stacked_sha = get(&gh, &format!("{REPO}/git/ref/heads/feat/issue-{stacked}"))
        .map(|answer| answer["object"]["sha"].as_str().map(str::to_owned));
    let comments = get_all(
        &gh,
        &format!("{REPO}/issues/{stacked}/comments?per_page=100"),
    );
    cleanup(&gh, epic, blocker, stacked, pr).expect("issues closed");

    let base = format!("feat/issue-{blocker}");
    match refused {
        Err(Error::Refused(message)) => assert_eq!(
            message, "1 open issues block it. Build it only after they close",
            "plain claim refuses"
        ),
        other => panic!("plain claim is refused, got {other:?}"),
    }
    let listed = ready_stacked.expect("ready --stacked");
    let entry = ready_entry(&listed, stacked).expect("ready json");
    assert_eq!(
        entry.as_ref().and_then(|e| e["stacked"]["base"].as_str()),
        Some(base.as_str()),
        "ready --stacked lists the stacked task with its base"
    );
    let plain = ready_plain.expect("ready");
    assert!(
        ready_entry(&plain, stacked).expect("ready json").is_none(),
        "plain ready does not list it"
    );
    let text = claimed.expect("claim --stacked");
    assert_eq!(
        text.lines().last(),
        Some(format!("base {base}").as_str()),
        "claim --stacked names the base"
    );
    assert_eq!(
        stacked_sha.expect("stacked branch").as_deref(),
        Some(sha.as_str()),
        "stacked branch starts at the blocker commit"
    );
    let marker = format!("<!-- taskrock:stacked-on {base} -->");
    let comments = comments.expect("comments");
    assert!(
        comments
            .iter()
            .any(|c| c["body"].as_str().is_some_and(|b| b.contains(&marker))),
        "marker comment is posted"
    );
}
