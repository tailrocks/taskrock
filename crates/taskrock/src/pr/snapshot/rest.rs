//! The six REST reads of a snapshot.

use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::{CheckFact, CodexFact, PullSnapshot, PullState, QueueFact, VerdictFact};
use crate::error::Error;
use crate::gh::Gh;
use crate::github::{get, get_all, is_status};
use crate::link::PullLink;

/// The size of one page of a read that returns an object, so `get_all` cannot page it.
const PAGE: u64 = 100;
/// The bot that posts the state of its review as a hidden comment.
const CODEX_BOT: &str = "chatgpt-codex-connector[bot]";
const CODEX_MARKER: &str = "<!-- codex-review-state:v1 ";
/// The start of the line that names the reviewer in the review of the manager.
const REVIEWER: &str = "Reviewer:";

/// What the branch rules of the base branch say.
#[derive(Debug, Default)]
pub(super) struct Rules {
    pub required_checks: Vec<String>,
    pub merge_queue: bool,
}

#[derive(Deserialize)]
struct Pull {
    number: u64,
    node_id: String,
    state: String,
    #[serde(default)]
    merged: bool,
    merge_commit_sha: Option<String>,
    #[serde(default)]
    draft: bool,
    body: Option<String>,
    mergeable: Option<bool>,
    mergeable_state: Option<String>,
    base: Base,
    head: Head,
}

#[derive(Deserialize)]
struct Base {
    #[serde(rename = "ref")]
    name: String,
    repo: Option<Repo>,
}

#[derive(Deserialize)]
struct Head {
    #[serde(rename = "ref")]
    name: String,
    sha: String,
    repo: Option<Repo>,
}

#[derive(Deserialize)]
struct Repo {
    full_name: String,
    default_branch: Option<String>,
}

#[derive(Deserialize)]
struct Runs {
    total_count: u64,
    check_runs: Vec<Run>,
}

#[derive(Deserialize)]
struct Run {
    id: u64,
    name: String,
    status: String,
    conclusion: Option<String>,
}

#[derive(Deserialize)]
struct Combined {
    total_count: u64,
    statuses: Vec<Status>,
}

#[derive(Deserialize)]
struct Status {
    context: String,
    state: String,
}

#[derive(Deserialize)]
struct Rule {
    #[serde(rename = "type")]
    kind: String,
    parameters: Option<Parameters>,
}

#[derive(Deserialize)]
struct Parameters {
    #[serde(default)]
    required_status_checks: Vec<Required>,
}

#[derive(Deserialize)]
struct Required {
    context: String,
}

/// A review or an issue comment.
#[derive(Deserialize)]
struct Note {
    body: Option<String>,
    submitted_at: Option<String>,
    user: Option<User>,
}

#[derive(Deserialize)]
struct User {
    login: String,
}

#[derive(Deserialize)]
struct State {
    code_review: Review,
    security_review: Review,
}

#[derive(Deserialize)]
struct Review {
    commit_sha: String,
    status: String,
}

fn decode<T: DeserializeOwned>(value: serde_json::Value) -> Result<T, Error> {
    serde_json::from_value(value).map_err(|error| Error::Json(error.to_string()))
}

fn decode_all<T: DeserializeOwned>(values: Vec<serde_json::Value>) -> Result<Vec<T>, Error> {
    values.into_iter().map(decode).collect()
}

/// Read 1: `pulls/<n>`. The lists of the snapshot are empty.
pub(super) fn pull(gh: &dyn Gh, link: &PullLink) -> Result<PullSnapshot, Error> {
    let pull: Pull = decode(get(gh, &link.api_path())?)?;
    let base = pull
        .base
        .repo
        .ok_or_else(|| Error::Json("pulls: the base has no repository".to_owned()))?;
    let cross_repository = pull
        .head
        .repo
        .as_ref()
        .is_none_or(|repo| repo.full_name != base.full_name);
    let state = match (pull.state.as_str(), pull.merged) {
        ("open", _) => PullState::Open,
        (_, true) => PullState::Merged,
        _ => PullState::Closed,
    };
    Ok(PullSnapshot {
        number: pull.number,
        node_id: pull.node_id,
        state,
        merge_commit: pull.merge_commit_sha.filter(|_| state == PullState::Merged),
        draft: pull.draft,
        body: pull.body.unwrap_or_default(),
        base_ref: pull.base.name,
        head_ref: pull.head.name,
        head_sha: pull.head.sha,
        default_branch: base.default_branch.unwrap_or_default(),
        cross_repository,
        mergeable: pull.mergeable,
        mergeable_state: pull.mergeable_state.unwrap_or_else(|| "unknown".to_owned()),
        checks: Vec::new(),
        required_checks: Vec::new(),
        verdicts: Vec::new(),
        threads: Vec::new(),
        queue: QueueFact {
            has_queue: false,
            entry: None,
        },
        codex: None,
        problems: Vec::new(),
        threads_unread: Vec::new(),
    })
}

/// Read 2: the check runs of the head commit. The answer is an object.
pub(super) fn check_runs(
    gh: &dyn Gh,
    link: &PullLink,
    head: &str,
    problems: &mut Vec<String>,
) -> Result<Vec<CheckFact>, Error> {
    let path = format!(
        "{}/commits/{head}/check-runs?per_page={PAGE}",
        link.repo_path()
    );
    let runs: Runs = decode(get(gh, &path)?)?;
    if runs.total_count > PAGE {
        problems.push(format!(
            "The head commit has {} check runs and only {PAGE} were read.",
            runs.total_count
        ));
    }
    // A second workflow run on the head, such as the one a `ready_for_review` event starts, is a
    // second check suite, so a name can come back more than once. Only its newest run, the one
    // with the highest id, counts, as in `gh pr checks`. The name keeps its first place.
    let mut facts: Vec<(u64, CheckFact)> = Vec::new();
    for run in runs.check_runs {
        let fact = CheckFact {
            completed: run.status == "completed",
            name: run.name,
            conclusion: run.conclusion,
        };
        match facts.iter_mut().find(|(_, seen)| seen.name == fact.name) {
            Some(slot) if slot.0 < run.id => *slot = (run.id, fact),
            Some(_) => {}
            None => facts.push((run.id, fact)),
        }
    }
    Ok(facts.into_iter().map(|(_, fact)| fact).collect())
}

/// Read 3: the combined commit status of the head commit.
pub(super) fn statuses(
    gh: &dyn Gh,
    link: &PullLink,
    head: &str,
    problems: &mut Vec<String>,
) -> Result<Vec<CheckFact>, Error> {
    let path = format!("{}/commits/{head}/status?per_page={PAGE}", link.repo_path());
    let combined: Combined = decode(get(gh, &path)?)?;
    if combined.total_count > combined.statuses.len() as u64 {
        problems.push(format!(
            "The head commit has {} commit statuses and only {} were read.",
            combined.total_count,
            combined.statuses.len()
        ));
    }
    let mut facts = Vec::new();
    for status in combined.statuses {
        let (completed, conclusion) = match status.state.as_str() {
            "success" => (true, Some("success")),
            "failure" | "error" => (true, Some("failure")),
            "pending" => (false, None),
            other => {
                problems.push(format!(
                    "The commit status {} has the unknown state {other}.",
                    status.context
                ));
                (false, None)
            }
        };
        facts.push(CheckFact {
            name: status.context,
            completed,
            conclusion: conclusion.map(str::to_owned),
        });
    }
    Ok(facts)
}

/// Read 4: the rules of the base branch. No access or no rules is not an error.
pub(super) fn rules(gh: &dyn Gh, link: &PullLink, base: &str) -> Result<Rules, Error> {
    let path = format!("{}/rules/branches/{base}", link.repo_path());
    let values = match get_all(gh, &path) {
        Ok(values) => values,
        Err(error) if is_status(&error, 403) || is_status(&error, 404) => {
            return Ok(Rules::default());
        }
        Err(error) => return Err(error),
    };
    let mut found = Rules::default();
    for rule in decode_all::<Rule>(values)? {
        match rule.kind.as_str() {
            "merge_queue" => found.merge_queue = true,
            "required_status_checks" => found.required_checks.extend(
                rule.parameters
                    .into_iter()
                    .flat_map(|p| p.required_status_checks)
                    .map(|r| r.context),
            ),
            _ => {}
        }
    }
    found.required_checks.sort();
    found.required_checks.dedup();
    Ok(found)
}

/// Read 5: the verdicts in the reviews. The only source of a verdict: every agent posts as one
/// account, so an issue comment could not be told from a reviewer's word.
pub(super) fn reviews(gh: &dyn Gh, link: &PullLink) -> Result<Vec<VerdictFact>, Error> {
    let path = format!("{}/reviews?per_page={PAGE}", link.api_path());
    let notes = decode_all::<Note>(get_all(gh, &path)?)?;
    Ok(notes
        .iter()
        .filter_map(|note| verdict(note.body.as_deref()?, note.submitted_at.as_deref()?))
        .collect())
}

/// Read 6: the Codex state in the issue comments.
pub(super) fn comments(
    gh: &dyn Gh,
    link: &PullLink,
    problems: &mut Vec<String>,
) -> Result<Option<CodexFact>, Error> {
    let path = format!(
        "{}/issues/{}/comments?per_page={PAGE}",
        link.repo_path(),
        link.number
    );
    let notes = decode_all::<Note>(get_all(gh, &path)?)?;
    let mut codex = None;
    for note in notes.iter().filter(|note| {
        note.user
            .as_ref()
            .is_some_and(|user| user.login == CODEX_BOT)
    }) {
        match note.body.as_deref().and_then(codex_state) {
            Some(Ok(fact)) => codex = Some(fact),
            Some(Err(error)) => {
                problems.push(format!("The Codex review state does not parse: {error}."));
            }
            None => {}
        }
    }
    Ok(codex)
}

/// The Codex state in a comment body: `None` when it has no state.
fn codex_state(body: &str) -> Option<Result<CodexFact, Error>> {
    let (_, after) = body.split_once(CODEX_MARKER)?;
    let json = after.split_once(" -->").map_or(after, |(json, _)| json);
    let state: Result<State, Error> =
        serde_json::from_str(json).map_err(|error| Error::Json(error.to_string()));
    Some(state.map(|state| CodexFact {
        code_sha: state.code_review.commit_sha,
        code_status: state.code_review.status,
        security_sha: state.security_review.commit_sha,
        security_status: state.security_review.status,
    }))
}

/// The verdict of a review body that has a line starting with `Reviewer:` and whose last
/// non-blank line is `VERDICT: PASS head=<40 hex>` or `VERDICT: FAIL head=<40 hex>`. The head is
/// lowercased, so that it equals a SHA of GitHub.
pub(super) fn verdict(body: &str, at: &str) -> Option<VerdictFact> {
    body.lines().find(|line| line.starts_with(REVIEWER))?;
    let line = body.lines().map(str::trim).rfind(|line| !line.is_empty())?;
    let (word, head) = line.strip_prefix("VERDICT: ")?.split_once(" head=")?;
    let pass = match word {
        "PASS" => true,
        "FAIL" => false,
        _ => return None,
    };
    let sha = head.len() == 40 && head.bytes().all(|byte| byte.is_ascii_hexdigit());
    sha.then(|| VerdictFact {
        pass,
        head: head.to_ascii_lowercase(),
        at: at.to_owned(),
    })
}
