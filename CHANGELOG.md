# Changelog

All notable changes to this project are recorded in this file.

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
