//! Claim a task on GitHub: the branch, the statuses and the race.

use std::sync::Barrier;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::json;

use taskrock::claim::claim;
use taskrock::error::Error;
use taskrock::gh::{Gh as _, RealGh};
use taskrock::github::items::project_items;
use taskrock::github::send;
use taskrock::link::ProjectLink;

const REPO: &str = "repos/tailrocks/taskrock-sandbox";

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

fn create_issues(gh: &RealGh, stamp: u64) -> Result<(u64, u64), Error> {
    let epic = send(
        gh,
        "POST",
        &format!("{REPO}/issues"),
        &json!({"title": format!("[claim test] epic {stamp}"), "type": "Epic"}),
    )?;
    let task = send(
        gh,
        "POST",
        &format!("{REPO}/issues"),
        &json!({"title": format!("[claim test] task {stamp}"), "type": "Task", "parent_issue_id": epic["id"]}),
    )?;
    Ok((number(&epic)?, number(&task)?))
}

fn add_and_wait(gh: &RealGh, epic: u64, task: u64) -> Result<(), Error> {
    send(
        gh,
        "POST",
        "orgs/tailrocks/projectsV2/10/items",
        &json!({"type": "Issue", "owner": "tailrocks", "repo": "taskrock-sandbox", "number": epic}),
    )?;
    let deadline = Instant::now() + Duration::from_secs(280);
    loop {
        let items = project_items(gh, &project())?;
        let has = |n: u64| items.iter().any(|item| item.link.number == n);
        if has(epic) && has(task) {
            return Ok(());
        }
        if Instant::now() > deadline {
            return Err(Error::Project("both issues join project 10".to_owned()));
        }
        std::thread::sleep(Duration::from_secs(5));
    }
}

fn race(gh: &RealGh, url: &str) -> Vec<Result<String, Error>> {
    let barrier = Barrier::new(4);
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    claim(gh, url)
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|worker| {
                worker.join().unwrap_or_else(|_| {
                    Err(Error::Gh {
                        args: "claim".to_owned(),
                        stderr: "a worker stopped".to_owned(),
                    })
                })
            })
            .collect()
    })
}

fn status_of(gh: &RealGh, issue: u64) -> Result<Option<String>, Error> {
    let items = project_items(gh, &project())?;
    items
        .iter()
        .find(|item| item.link.number == issue)
        .map_or_else(
            || Err(Error::Project("the issue is an item".to_owned())),
            |item| Ok(item.status.clone()),
        )
}

fn cleanup(gh: &RealGh, epic: u64, task: u64) -> Result<(), Error> {
    let branch = format!("{REPO}/git/refs/heads/feat/issue-{task}");
    match gh.run(&["api", "-X", "DELETE", &branch], None) {
        Ok(_) | Err(_) => {}
    }
    for issue in [epic, task] {
        send(
            gh,
            "PATCH",
            &format!("{REPO}/issues/{issue}"),
            &json!({"state": "closed"}),
        )?;
    }
    Ok(())
}

#[test]
#[ignore = "talks to GitHub"]
fn four_claims_leave_one_winner() {
    let gh = RealGh::default();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_secs();
    let (epic, task) = create_issues(&gh, stamp).expect("issues created");
    add_and_wait(&gh, epic, task).expect("both issues join project 10");
    let url = format!("https://github.com/tailrocks/taskrock-sandbox/issues/{task}");
    let outcomes = race(&gh, &url);
    let task_status = status_of(&gh, task).expect("task status");
    let epic_status = status_of(&gh, epic).expect("epic status");
    cleanup(&gh, epic, task).expect("issues closed");
    assert_eq!(
        outcomes.iter().filter(|o| o.is_ok()).count(),
        1,
        "one claim wins"
    );
    let refused = outcomes
        .iter()
        .filter(|o| matches!(o, Err(Error::Refused(_))))
        .count();
    assert_eq!(refused, 3, "three claims are refused");
    assert_eq!(task_status.as_deref(), Some("In Progress"), "task moves");
    assert_eq!(epic_status.as_deref(), Some("In Progress"), "epic moves");
}
