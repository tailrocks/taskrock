# Changelog

All notable changes to this project are recorded in this file.

## 0.5.2

- `pr merge` retargets the open pull requests based on the head branch only after the pull request is merged. Before, it retargeted first, so a merge that GitHub refused left the children on the default branch with the commits of their base. A refused merge and a queued pull request retarget nothing: a queued report, with the command to run again, says that the children are retargeted when the merge lands, and `pr merge` run again on the merged pull request reports `already_merged` and retargets the children that are still open. It works whether or not GitHub deleted the head branch: with `delete_branch_on_merge` on, GitHub retargets the children itself and `pr merge` finds none. The children go to the branch the pull request merged into, not always the default branch: a pull request that was merged into the branch of its blocker retargets its children there. A retarget that fails after the merge landed keeps the outcome `merged` and the exit code 0, and prints a line that names the child and the error. The `--json` key `retargeted` is empty unless the outcome is `merged` or `already_merged`. The `--json` report has two more always-present keys, filled on those two outcomes: `retarget_failures`, an array of `{number, error}` for each child that GitHub would not retarget, and `retarget_error`, the error of the listing of the children or null. They are additive, so `schema` stays 1.

## 0.5.1

- `pr status` counts only the newest run of each check name on the head. A failed run that a later run of the same check replaced no longer blocks the pull request.

## 0.5.0

- `pr status <link>` prints the verdict of a pull request: READY, or each reason that it is not. `--json` prints the same verdict as JSON. The exit code is 0 when the pull request is done (READY or MERGED), 10 when it waits for something that changes on its own, and 20 when it is blocked until someone acts.
- `pr threads <link>` replies to and resolves the review threads of bots. `--defer` settles every unresolved deferrable thread, a thread of severity P3. `--resolve <id> --reason <text>` settles the one thread `<id>`. The command refuses to write when it could not read every thread, or when the pull request changed since it was read.
- `pr merge <link>` merges a READY pull request at the head that `pr status` judged. It points each open pull request based on this pull request's head branch at the default branch, then queues the pull request when the base has a merge queue, and otherwise squash-merges it. The exit code is 0 when merged, 10 when queued, and 20 when GitHub refuses the merge. A pull request from a fork is refused with exit code 3.
- A pull request with no check on its head is not READY. `pr status` waits with `No check has started on the head yet.`
- `[review] max_rounds` counts the reviewed heads of a pull request: the distinct heads that have a review verdict. In 0.4.0 `check` only required it to be at least 1, and no command acted on it. `pr status` now blocks the pull request for the owner when the count reaches the limit and no PASS verdict names the current head. The default is 3.
- New key `[review] codex`: a list of the repository names of the owner where the Codex connector reviews pull requests. `pr status` waits for the Codex code and security review of the head of a ready, non-draft pull request in a listed repository, even before the connector posts its comment. The default is an empty list. `check` refuses an empty name, a name with a `/`, and a name that is listed twice.
- A GraphQL `errors` array from GitHub is reported as `GitHub GraphQL error: <messages>`, with the messages joined by `; `. A failure of `gh` itself, such as no answer, an HTTP error status or bad credentials, keeps the message of `gh`.
- The layout test, `crates/taskrock/tests/layout.rs`, checks the unit-test files under `src` against `docs/structure.md`. Each test file is listed there, and each test module is declared under `#[cfg(test)]`. It runs with `cargo nextest run --workspace --all-features --locked -E 'binary(layout)'`.

## 0.4.0

- Breaking: check refuses an issue that is not an Initiative and has no parent.
- `check` and `apply` refuse an issue that has no `parent` unless it is an Initiative.
- `check` and `apply` answer a `parent` or `blocked_by` value that is a bare issue number, like `2878` or `#2878`, with the link to write: `https://github.com/<owner>/<repo>/issues/<number>`.
- `apply` gives an issue filed under a link `parent` the milestone of that parent when the issue and `plan.toml` name none. The milestone must exist in the issue's repository: `apply` never creates it and stops before any `POST` when it is missing. A dry run says `would set milestone <title> on <key>, from its parent <link>`.
- `apply` stops before any `POST` when a link `parent` has no room for the new issues that name it: GitHub allows 100 sub-issues per parent. The message is `<link> has <n> sub-issues and GitHub allows 100: file under a child Epic of it`.
- `ready <issue link>` lists the open Task, Bug, Feature and Epic items of the link's milestone that have no parent in a `LOOSE <n>` section and in the `--json` key `loose`, so a manager sees an issue that was filed outside its tree. A loose item is not a candidate and not a problem. A project link or a link with no milestone has none.
- Tests remove the folders they create: a unit test no longer leaves `taskrock-*` folders in TMPDIR (#163).
- `pr`: a preview command group, `pr status`, `pr threads` and `pr merge`. The exit codes 10 and 20 are defined, and no command returns them yet. Each subcommand is a stub: it writes `error: taskrock pr <name> is not built yet` to standard error and exits 1.

## 0.3.0

- `ready --stacked` also lists a Todo task whose open blockers form one stack: each is In Progress with an open pull request from `feat/issue-<blocker>`. The text line `stacked on:` and the `--json` key `stacked` (`base`, `blocker`, `pull_request`) name the branch to start from. Every ready entry of `--json` has the key, `null` for a task that is not stacked.
- `claim --stacked` takes such a task: it creates `feat/issue-<number>` from the branch of the top blocker, posts a comment with the marker `<!-- taskrock:stacked-on feat/issue-<blocker> -->`, and prints a third line `base <branch>`. With no open blocker it starts from the default branch.
- `claim --stacked` refuses a blocker in another repository, a blocker that is not In Progress, a blocker with no open pull request, and open blockers that are not one stack.
- The design accepts stacked pull requests at the owner's request in https://github.com/tailrocks/taskrock/issues/139. A stacked task is still one branch and one pull request.
- A live sandbox test, `sandbox_stacked`, proves the stacked claim on GitHub.

## 0.2.1

- `apply`: `parent` and `blocked_by` in a draft may be a full issue link to an issue that already exists, so one draft files a Bug under an existing epic.
- `apply`: `plan.toml` has an optional `milestone`, set on every created issue that names no milestone of its own.
- `apply` puts in Triage only the Bugs and Features that it creates, or that have no Status. An issue that exists keeps its Status.
- `ready`: a `TRIAGE <n>` section and a `triage` key in `--json` list the open Triage issues under the link. They are not candidates and not problems.
- `ready` reports "has children" only for a Task or a Bug, and gives the blocker-cycle hint only when a candidate is blocked.
- `check` refuses a title that starts with a lowercase letter, unless it starts with the name of the repository.
- `claim` names the skill `taskrock-triage` for an issue in Triage, and says to wait for open blockers.
- `setup` changes the layout of a view whose layout differs.
- A GraphQL error from `gh` comes back as its messages joined by `; `.

## 0.2.0

- The skills moved to the plugin `taskrock` in `tailrocks/taskrock-plugins`, which is its own marketplace for Claude Code, Codex, Grok Build, Muse Code and Antigravity. The package `tailrocks/tailrocks-taskrock-skills` is replaced.
- `setup`: make GitHub match `<owner>/.github/taskrock.toml`: issue types, routing fields, projects, Status, views, repository links, milestones and workflows.
- The config file `taskrock.toml`, checked by `check <file>`.
- Issue formats per type from the config. The Feature type. The Triage status for Bug and Feature.
- `check`: Done-when boxes need a command and a result, bare issue numbers are refused, `--coverage` and `--config`.
- `template`: print an empty body of a type.
- `edit`: replace a body, with frozen sections after In Progress.
- `release --note`: post a handoff comment.
- `apply`: Triage and Todo, and the portfolio project of `plan.toml`.
- `ready` and `claim`: check the build sections. `ready` warns about tasks that name the same file and refuses a portfolio project.
- Delivery and portfolio projects, told apart by a README marker.
- The account setup script is replaced by `setup`.

## 0.1.0

- `check`: check a draft against the rules, using no network.
- `apply`: create the plan of a draft on GitHub.
- `ready`: list the tasks that can start now.
- `claim`: take a task.
- `release`: give a task back.
- The `taskrock-goal`, `taskrock-issue` and `taskrock-plan` skills.
