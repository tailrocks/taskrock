//! Apply a draft twice: the second run creates nothing.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use taskrock::apply::{Options, create_issues, run};
use taskrock::error::Error;
use taskrock::gh::RealGh;
use taskrock::github::items::project_items;
use taskrock::github::{get, send};
use taskrock::link::ProjectLink;

const REPO: &str = "tailrocks/taskrock-sandbox";

/// The tests share one repository and GitHub lists new issues late, so they
/// take this lock and create their issues one after another.
static SANDBOX: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn folder(stamp: u64) -> PathBuf {
    std::env::temp_dir().join(format!("taskrock-apply-{stamp}"))
}

fn write_file(folder: &Path, name: &str, text: &str) -> Result<(), Error> {
    std::fs::write(folder.join(name), text).map_err(|error| Error::Draft {
        file: name.to_owned(),
        message: error.to_string(),
    })
}

/// Writes the draft: the plan, the epic, and one task file per `tasks` entry
/// of `(key, title, blocked_by line or "")`.
fn write_draft(
    folder: &Path,
    id: &str,
    stamp: u64,
    tasks: &[(&str, String, &str)],
) -> Result<(), Error> {
    std::fs::create_dir_all(folder).map_err(|error| Error::Draft {
        file: folder.display().to_string(),
        message: error.to_string(),
    })?;
    write_file(
        folder,
        "plan.toml",
        &format!(
            "id = \"{id}\"\nrepo = \"{REPO}\"\nproject = \"https://github.com/orgs/tailrocks/projects/10\"\n"
        ),
    )?;
    write_file(
        folder,
        "E1.md",
        &format!(
            "+++\nkey = \"E1\"\ntitle = \"[apply test] epic {stamp}\"\ntype = \"Epic\"\n+++\n## Outcome\n\nUsers export the summary report.\n\n## Done when\n\n- [ ] The summary report downloads.\n"
        ),
    )?;
    for (key, title, blocked) in tasks {
        write_file(
            folder,
            &format!("{key}.md"),
            &format!(
                "+++\nkey = \"{key}\"\ntitle = \"{title}\"\ntype = \"Task\"\nparent = \"E1\"\n{blocked}agent = \"tailrocks-systems-engineer\"\nruntime = \"muse\"\nmodel = \"muse-spark-1.3\"\neffort = \"high\"\nreview_agent = \"tailrocks-code-reviewer\"\nreview_runtime = \"codex\"\nreview_model = \"muse-spark-1.3\"\nreview_effort = \"medium\"\n+++\n## Outcome\n\nThe report command renders rows.\n\n## Decisions already made\n\n- The delimiter is a comma.\n\n## Steps\n\n1. Render the rows.\n\n## Done when\n\n- [ ] `report --format csv` prints the rows.\n\n## Out of scope\n\n- No other format changes.\n"
            ),
        )?;
    }
    Ok(())
}

fn close(gh: &RealGh, numbers: &[u64]) -> Result<(), Error> {
    for number in numbers {
        send(
            gh,
            "PATCH",
            &format!("repos/{REPO}/issues/{number}"),
            &json!({"state": "closed"}),
        )?;
    }
    Ok(())
}

#[test]
#[ignore = "talks to GitHub"]
fn second_apply_creates_nothing() {
    let _guard = SANDBOX.lock().expect("sandbox lock");
    let gh = RealGh::default();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_secs();
    let folder = folder(stamp);
    let id = format!("apply-{stamp}");
    write_draft(
        &folder,
        &id,
        stamp,
        &[
            ("T01", format!("[apply test] first task {stamp}"), ""),
            (
                "T02",
                format!("[apply test] second task {stamp}"),
                "blocked_by = [\"T01\"]\n",
            ),
        ],
    )
    .expect("draft is written");
    let options = Options {
        dry_run: false,
        pause: Duration::from_secs(1),
        poll: Duration::from_secs(5),
    };
    let (_, first, first_lines) = create_issues(&gh, &folder, &options).expect("first apply");
    let (_, second, second_lines) = create_issues(&gh, &folder, &options).expect("second apply");
    assert_eq!(first_lines.len(), 3, "three issues are created");
    assert!(
        first_lines.iter().all(|line| line.starts_with("created ")),
        "first run creates: {first_lines:?}"
    );
    assert!(
        second_lines.iter().all(|line| line.starts_with("exists ")),
        "second run creates nothing: {second_lines:?}"
    );
    let number = second.get("T01").expect("T01 is mapped").link.number;
    let issue: Value = get(&gh, &format!("repos/{REPO}/issues/{number}")).expect("task reads back");
    assert_eq!(
        issue.pointer("/type/name").and_then(Value::as_str),
        Some("Task"),
        "the task has its type"
    );
    assert!(
        issue["parent_issue_url"].is_string(),
        "the task has a parent"
    );
    assert_eq!(
        issue["issue_field_values"].as_array().map(Vec::len),
        Some(8),
        "the task has eight routing keys"
    );
    let numbers: Vec<u64> = ["E1", "T01", "T02"]
        .into_iter()
        .map(|key| first.get(key).expect("key is mapped").link.number)
        .collect();
    close(&gh, &numbers).expect("issues are closed");
    std::fs::remove_dir_all(&folder).expect("the draft folder is removed");
}

/// The numbers of the issues the first run created, in creation order.
fn created_numbers(text: &str) -> Vec<u64> {
    text.lines()
        .filter_map(|line| line.strip_prefix("created #"))
        .filter_map(|rest| rest.split_whitespace().next())
        .filter_map(|number| number.parse().ok())
        .collect()
}

#[test]
#[ignore = "talks to GitHub"]
fn second_apply_links_nothing_twice() {
    let _guard = SANDBOX.lock().expect("sandbox lock");
    let gh = RealGh::default();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_secs();
    let folder = std::env::temp_dir().join(format!("taskrock-apply-blockers-{stamp}"));
    let id = format!("apply-blockers-{stamp}");
    write_draft(
        &folder,
        &id,
        stamp,
        &[
            ("T01", format!("[apply test] first task {stamp}"), ""),
            (
                "T02",
                format!("[apply test] second task {stamp}"),
                "blocked_by = [\"T01\"]\n",
            ),
            (
                "T03",
                format!("[apply test] third task {stamp}"),
                "blocked_by = [\"T01\"]\n",
            ),
        ],
    )
    .expect("draft is written");
    let options = Options {
        dry_run: false,
        pause: Duration::from_secs(1),
        poll: Duration::from_secs(5),
    };
    let first = run(&gh, &folder, &options).expect("first apply");
    let second = run(&gh, &folder, &options).expect("second apply");
    for (name, text) in [("first", &first), ("second", &second)] {
        assert_eq!(
            text.lines().last(),
            Some("DONE 4 issues, 2 blockers"),
            "{name} run ends with DONE: {text}"
        );
    }
    assert!(
        second.lines().all(|line| !line.starts_with("created ")
            && !line.starts_with("blocked ")
            && !line.starts_with("project: added ")),
        "second run links nothing twice: {second}"
    );
    let numbers = created_numbers(&first);
    assert_eq!(numbers.len(), 4, "four issues are created: {first}");
    let project = ProjectLink {
        owner: "tailrocks".to_owned(),
        number: 10,
    };
    let items = project_items(&gh, &project).expect("items read back");
    for number in &numbers {
        let item = items
            .iter()
            .find(|item| item.link.number == *number)
            .expect("issue is in the project");
        assert_eq!(
            item.status.as_deref(),
            Some("Todo"),
            "the new item has status Todo"
        );
    }
    close(&gh, &numbers).expect("issues are closed");
    std::fs::remove_dir_all(&folder).expect("the draft folder is removed");
}
