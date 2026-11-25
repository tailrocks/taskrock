# Structure

taskrock is one Rust crate at `crates/taskrock`. It holds a library and the `taskrock` command line. The agent skills that call the command live in a second repository, and the GitHub organization holds the issue types, fields and project that the command reads and writes. This page shows where each part lives and how one piece of work moves through them. It describes version 0.1.0 as of 2026-11-25.

## The parts

| Part | Where it lives | What it holds |
|---|---|---|
| The command | `tailrocks/taskrock`, crate `crates/taskrock` | The `taskrock` binary and its library. It has the commands `check`, `apply`, `ready`, `claim` and `release`. |
| The skills | `tailrocks/tailrocks-taskrock-skills`, folder `skills/` | Three skills: `taskrock-plan`, `taskrock-issue` and `taskrock-goal`. Each skill has a `SKILL.md` file. The package also holds the adapters for each agent host. |
| The marketplace | `tailrocks/tailrocks-skills` | The catalog that lists the skills package and selects its source commit. |
| The organization setup | The section "Set up an account" of [README](../README.md) | A script that creates the issue types `Initiative` and `Epic` and the eight routing fields. |
| The project | A project of the GitHub organization | The items of the plan, with the `Status` and `Parent issue` fields that taskrock reads and sets. A new project is a copy of one that has the fields. |
| The sandbox | `tailrocks/taskrock-sandbox` | The repository that the live tests use. |

## How the parts work together

1. The owner explains the work to an agent and starts the `taskrock-plan` skill. The skill writes a draft folder: a `plan.toml` file and one Markdown file for each issue.
2. The skill runs `taskrock check <folder>` until the check prints `OK`. It then runs `taskrock apply <folder> --dry-run` and `taskrock apply <folder>`. The apply step creates the issues with their parents, routing fields, milestones, blockers and project items. A second run creates nothing twice.
3. The `taskrock-goal` skill runs `taskrock ready <link> --json` to list the tasks that can start now. For each one it runs `taskrock claim <task link>`. The claim creates the branch `feat/issue-<number>` and sets the task to `In Progress`.
4. A builder, which runs the `taskrock-issue` skill, builds the task by its Steps and opens a pull request. The body of the pull request has `Closes #<number>`. When the task is wrong, the builder runs `taskrock release <link>` and gives the task back.
5. A reviewer runs the `Done when` commands again and posts a verdict. The manager merges the pull request, then closes each parent whose sub-issues are all closed.

The rules of the agents are in [AGENTS.md](../AGENTS.md). The reasons for this split are in [design](design.md).

## The crate

The workspace has one member, `crates/taskrock`. The file `src/main.rs` only parses the arguments with clap and calls `taskrock::run`. The library in `src/lib.rs` holds the logic. Its `run` function matches each command and calls the module that does it.

| Module | File or folder | What it does |
|---|---|---|
| `cli` | `src/cli.rs` | The clap types `Cli` and `Command`. There is one variant for each command. |
| `draft` | `src/draft.rs`, `src/draft/` | The draft in memory: `Plan`, `DraftIssue` and `Draft`. `draft::parse` reads a folder. `draft::check` checks it against the rules. |
| `apply` | `src/apply.rs`, `src/apply/` | Creates the issues of a draft on GitHub. Its children are `blockers`, `create`, `existing`, `fields`, `marker`, `milestones` and `project`. |
| `ready` | `src/ready.rs` | Sorts the open tasks into the ready, waiting on owner, in progress and blocked groups, and lists the problems of the plan. |
| `claim` | `src/claim.rs` | `claim` creates the branch and sets `In Progress`. `release` deletes the branch and sets `Todo`. |
| `link` | `src/link.rs` | Parses the links that a person pastes into `IssueLink`, `ProjectLink` and `Link`. It builds the URLs and API paths of each. |
| `fields` | `src/fields.rs` | The eight routing fields in the constant `ROUTING`, and `Routing`, the field values of one issue. |
| `github` | `src/github.rs`, `src/github/items.rs` | The REST helpers `get`, `get_all`, `send` and `is_status`. `items` reads the items of a project and sets their status. |
| `gh` | `src/gh.rs`, `src/gh/fake.rs` | The `Gh` trait, `RealGh` that runs the `gh` program, and `FakeGh` for tests. |
| `error` | `src/error.rs` | The `Error` type. A refusal exits with status 3. Every other error exits with status 1. |

## How a command reaches GitHub

Take `taskrock ready <link>` as an example.

1. `main.rs` parses the arguments into `cli::Cli` and calls `taskrock::run` with `gh::RealGh::default()`.
2. `lib.rs` matches `Command::Ready` and calls `ready::run`. That function turns the text into a `Link`.
3. `ready::run` reads the project items through `github::items::project_items`. That function reads the REST API through `github::get_all`.
4. Each read ends in `Gh::run`. `RealGh` starts the `gh` program. When GitHub reports a rate limit, it waits 60 seconds and tries again up to five times.
5. The command returns its text, and `lib.rs` prints it.

The `Gh` trait exists for the tests. `FakeGh` answers each call from a script and keeps a record of the calls, so a test runs without a network. `FakeGh` is compiled into the library and not only for tests, so the files in `crates/taskrock/tests/` can use it.

## The tests

There are three kinds of tests.

- Unit tests sit in `src/<module>/tests.rs`. The parent module declares them with the attribute `#[cfg(test)]`. The `claim` module also has `src/claim/release_tests.rs`.
- Integration tests sit in `crates/taskrock/tests/`. `cli.rs` runs the built binary with `--version`. `ready.rs` runs `ready::run` against the fixtures through `FakeGh`.
- Live tests are the four files whose names start with `sandbox_`. They call GitHub. Each test has the attribute `#[ignore = "talks to GitHub"]`, so a normal test run skips it. The live tests of apply and claim write to `tailrocks/taskrock-sandbox`.

The folder `crates/taskrock/fixtures/` holds the JSON answers of GitHub for the project tests, `project_fields.json` and `project_items.json`. It also holds two draft folders. `draft-good` is read by the draft and check tests. `draft-bad` holds files that break the rules. No test reads it by name.

## A draft

A draft is a folder. `plan.toml` holds `id`, `repo`, `project` and one `[[milestones]]` table for each milestone. Each other `.md` file is one issue. An issue file starts with TOML between two `+++` lines, and the Markdown body follows. The parser rejects any key that it does not know. This is a shortened issue from `fixtures/draft-good`:

```
+++
key = "T01"
title = "Add CSV rendering to the report command"
type = "Task"
parent = "E1"
milestone = "M1 Export"
agent = "tailrocks-systems-engineer"
+++
## Outcome
```

The issue keys are `key`, `title`, `type`, `repo`, `parent`, `blocked_by`, `milestone`, `agent`, `runtime`, `model`, `effort`, `review_agent`, `review_runtime`, `review_model` and `review_effort`. The type is `Initiative`, `Epic`, `Task` or `Bug`. A task has five sections in its body: `Outcome`, `Decisions already made`, `Steps`, `Done when` and `Out of scope`. An epic or an initiative needs `Outcome` and `Done when`.

## The checks and the generated CI

`mise run check` runs the format check, clippy, the tests, the doctests, the dependency checks, the policy check and the workflow check, as listed in [mise.toml](../mise.toml). The files in `.github/workflows/` are generated by `velnor-gen` from `.velnor/rust-ci.toml`, so a change goes into that file and never into a workflow. The workflow check fails when a generated file differs from the output of `velnor-gen`. The repository policy is `.alint.yml`. It extends a shared Rust policy at a pinned version, and it adds one rule that each `AGENTS.md` has a `CLAUDE.md` that points to it.
