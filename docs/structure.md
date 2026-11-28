# Structure

taskrock is one Rust crate at `crates/taskrock`. It holds a library and the `taskrock` command line. The agent skills that call the command live in a second repository, `tailrocks/taskrock-plugins`, and the GitHub organization holds the issue types, fields and projects that the command reads and writes. The command makes the organization match one config file, as [setup](setup.md) describes. This page shows where each part lives and how one piece of work moves through them. It describes version 0.2.1 as of 2026-12.

## The parts

| Part | Where it lives | What it holds |
|---|---|---|
| The command | `tailrocks/taskrock`, crate `crates/taskrock` | The `taskrock` binary and its library. It has the commands `check`, `apply`, `ready`, `claim`, `release`, `setup`, `template` and `edit`. |
| The skills | `tailrocks/taskrock-plugins`, folder `plugins/taskrock/skills/` | Five skills: `taskrock-plan`, `taskrock-issue`, `taskrock-goal`, `taskrock-triage` and `taskrock-setup`. Each skill has a `SKILL.md` file. The plugin also holds one manifest for each of the five agent CLIs. |
| The marketplace | `tailrocks/taskrock-plugins`, files `.claude-plugin/marketplace.json` and `.agents/plugins/marketplace.json` | The catalog that lists the plugin `taskrock` by its local path. Claude Code and Grok Build read the first file, Codex and Muse Code the second. Antigravity installs the plugin folder from a clone. |
| The config | `<owner>/.github/taskrock.toml` | The routing blocks, the issue formats, the projects with their repositories, workflows and shared milestones, of one organization. The views of each kind of project are fixed by the tool. |
| The organization setup | `taskrock setup <owner>` | Creates the missing issue types, the eight routing fields, the projects with their views, columns, links, milestones and workflows, from the config. It needs no browser and copies no project. |
| The projects | The projects of the GitHub organization | The items of the plans, with the `Status` and `Parent issue` fields that taskrock reads and sets. A delivery project holds the work. A portfolio project holds the initiatives. |
| The sandbox | `tailrocks/taskrock-sandbox` | The repository that the live tests use. |

## How the parts work together

1. The owner sets up the organization once: `taskrock setup <owner> --dry-run`, then `taskrock setup <owner>`. Then the owner explains the work to an agent and starts the `taskrock-plan` skill. The skill writes a draft folder: a `plan.toml` file and one Markdown file for each issue.
2. The skill runs `taskrock check <folder>` until the check prints `OK`. It then runs `taskrock apply <folder> --dry-run` and `taskrock apply <folder>`. The apply step creates the issues with their parents, routing fields, milestones, blockers and project items. A bug or a feature that is a report goes to Triage and every other issue to Todo. A second run creates nothing twice.
3. The `taskrock-goal` skill runs `taskrock ready <link> --json` to list the tasks that can start now. For each one it runs `taskrock claim <task link>`. The claim creates the branch `feat/issue-<number>` and sets the task to `In Progress`.
4. A builder, which runs the `taskrock-issue` skill, builds the task by its Steps and opens a pull request. The body of the pull request has `Closes #<number>`. When the task is wrong, the builder runs `taskrock release <link>` and gives the task back, with `--note` to leave a handoff comment. A body change goes through `taskrock edit`.
5. A reviewer runs the `Done when` commands again and answers PASS or FAIL. The manager posts the verdict as a comment on the pull request and merges it when the checks pass. Then it closes each parent whose sub-issues are all closed.

The rules of the agents are in [AGENTS.md](../AGENTS.md). The reasons for this split are in [design](design.md).

## The crate

The workspace has one member, `crates/taskrock`. The file `src/main.rs` only parses the arguments with clap and calls `taskrock::run`. The library in `src/lib.rs` holds the logic. Its `run` function matches each command and calls the module that does it.

| Module | File or folder | What it does |
|---|---|---|
| `cli` | `src/cli.rs` | The clap types `Cli` and `Command`. There is one variant for each command. |
| `config` | `src/config.rs`, `src/config/` | The config of an organization. `config::parse` reads a file, `config::check` checks it offline and is the command `check <file>`, and `config::load` reads `<owner>/.github/taskrock.toml` and falls back to `config::default` when the file is missing. |
| `format` | `src/format.rs` | The sections of an issue body. `format::section` reads one, `format::problems` checks a body against a format, and `format::bare_numbers` finds a bare `#123`. |
| `draft` | `src/draft.rs`, `src/draft/` | The draft in memory: `Plan`, `DraftIssue` and `Draft`. `draft::parse::read` reads a folder. `draft::check::check` checks it against the rules and the formats of a config, and `draft::check::run` is the command. Its children are `graph` and `body`. `draft::coverage` is the read-only report of `check --coverage`. |
| `apply` | `src/apply.rs`, `src/apply/` | Creates the issues of a draft on GitHub. Its children are `blockers`, `create`, `existing`, `fields`, `marker`, `milestones`, `project`, `status` and `portfolio`. `status` sets Triage or Todo, and `portfolio` adds the initiatives to the portfolio project. |
| `ready` | `src/ready.rs`, `src/ready/body.rs` | Sorts the open tasks into the ready, waiting on owner, in progress and blocked groups, and lists the problems of the plan. `body` reads the bodies, checks the build sections and finds the files that two tasks name. |
| `stack` | `src/stack.rs` | Decides where a stacked task starts. `stack::base` reads the open blockers of a task and the open pull request of each blocker's branch, and returns the top blocker of their one stack as a `Stack`, or refuses. |
| `claim` | `src/claim.rs` | `claim` creates the branch and sets the task and its open parents to `In Progress`. When the status change fails, it deletes the branch again. `release` sets the task to `Todo` and frees its branch. A branch with no commit ahead of the default branch is deleted. A branch with commits is renamed to `feat/issue-<number>-parked-<time>`, so the work is kept. A parent goes back to `Todo` when none of its children is in progress or done. `claim` refuses a task whose body breaks its build sections. `claim_stacked`, in `src/claim/stacked.rs`, starts the branch from the branch of the top open blocker, posts the stacked comment, and deletes the branch and the comment again when the status change fails. |
| `handoff` | `src/handoff.rs` | Posts the note of `release --note` as a comment. |
| `setup` | `src/setup.rs`, `src/setup/` | `taskrock setup`. `setup::run` reads the config and calls one module for each step, in order: `types`, `fields`, `projects`, `project_fields`, `views`, `repos`, `milestones` and `workflows`. |
| `projects` | `src/projects.rs` | The README marker of a project, the list of the projects of an organization and the kind of a project. |
| `template` | `src/template.rs` | Prints an empty body with the file sections of a type. |
| `edit` | `src/edit.rs` | Replaces the body of an issue after it checks the body against the type and the frozen sections. |
| `link` | `src/link.rs` | Parses the links that a person pastes into `IssueLink`, `ProjectLink` and `Link`. It builds the URLs and API paths of each. |
| `fields` | `src/fields.rs` | The eight routing fields in the constant `ROUTING`, and `Routing`, the field values of one issue. |
| `github` | `src/github.rs`, `src/github/items.rs` | The REST helpers `get`, `get_all`, `send` and `is_status`. `items` reads the items of a project and sets their status, and `project_of` finds the delivery project of an issue. |
| `gh` | `src/gh.rs`, `src/gh/fake.rs` | The `Gh` trait, `RealGh` that runs the `gh` program, and `FakeGh` for tests. |
| `error` | `src/error.rs` | The `Error` type. A refusal exits with status 3. Every other error exits with status 1. |

## How a command reaches GitHub

Take `taskrock ready <link>` as an example.

1. `main.rs` parses the arguments into `cli::Cli` and calls `taskrock::run` with `gh::RealGh::default()`.
2. `lib.rs` matches `Command::Ready` and calls `ready::run`. That function turns the text into a `Link`.
3. `ready::run` reads the project items through `github::items::project_items`. That function reads the REST API through `github::get_all`.
4. `ready::run` then reads the branches whose names start with `feat/issue-` in each repository, so a task that is already claimed is left out.
5. Each read ends in `Gh::run`. `RealGh` starts the `gh` program. When GitHub reports a rate limit, it waits 60 seconds and tries again up to five times.
6. The command returns its text, and `lib.rs` prints it.

The `Gh` trait exists for the tests. `FakeGh` answers each call from a script and keeps a record of the calls, so a test runs without a network. `FakeGh` is compiled into the library and not only for tests, so the files in `crates/taskrock/tests/` can use it.

## The tests

There are three kinds of tests.

- Unit tests sit in `src/<module>/tests.rs`. The parent module declares them with the attribute `#[cfg(test)]`. When a module's `tests.rs` would pass the 150-line cap, its tests move to sibling files named `src/<module>/<name>_tests.rs`. The parent module declares each sibling file with `#[cfg(test)] mod <name>_tests;`. These sibling files exist:
  - `src/apply/links/flow_tests.rs`
  - `src/apply/links/milestone_room_tests.rs`
  - `src/apply/links/milestone_tests.rs`
  - `src/apply/links/order_tests.rs`
  - `src/claim/body_tests.rs`
  - `src/claim/refuse_tests.rs`
  - `src/claim/release_tests.rs`
  - `src/draft/check/links_tests.rs`
  - `src/draft/check/root_tests.rs`
  - `src/draft/check/title_tests.rs`
  - `src/github/items/project_tests.rs`
  - `src/link/pull_tests.rs`
  - `src/pr/snapshot/case_tests.rs`
  - `src/pr/snapshot/facts_tests.rs`
  - `src/ready/loose_tests.rs`
  - `src/ready/problems_tests.rs`
  - `src/ready/stacked_tests.rs`
  - `src/ready/triage_tests.rs`
- Integration tests sit in `crates/taskrock/tests/`. `cli.rs` runs the built binary with `--version`, with `--help` and with no argument. `ready.rs` runs `ready::run` against the fixtures through `FakeGh`. `check_config.rs` runs `taskrock check` on a config file.
- Live tests are the files whose names start with `sandbox_`. They call GitHub. Each test has the attribute `#[ignore = "talks to GitHub"]`, so a normal test run skips it. The live tests of apply and claim write to `tailrocks/taskrock-sandbox`. The live test of setup, `sandbox_setup.rs`, runs `taskrock setup` twice on the sandbox project and expects no change the second time.

The folder `crates/taskrock/fixtures/` holds the JSON answers of GitHub for the project tests, `project_fields.json` and `project_items.json`. It also holds five draft folders. `draft-good` is read by the draft and check tests. `draft-triage` has a Task and a Bug, for the tests of the statuses that `apply` sets. `draft-portfolio` has an Initiative and a `portfolio` key in its `plan.toml`, for the tests of the portfolio step. `draft-links` has one Task whose `parent` and `blocked_by` are links to issues that already exist and a `milestone` in its `plan.toml`, for the tests of those keys. `draft-bad` holds files that break the rules. No test reads it by name. The config fixtures are `config-good.toml`, a complete config that passes `check`, `config-bad.toml`, which breaks three rules, and `config-sandbox.toml`, the config that the live test of setup applies to the sandbox project. The folder `fixtures/pr/` holds answers of GitHub recorded with `gh api` from pull requests of tailrocks/velnor-new, for the tests of `pr::snapshot`: `pull-`, `checks-`, `status-`, `reviews-`, `comments-` and `threads-` files for the pull requests 2932, 2963, 3033 and 3084 (3084 was in the merge queue), and `rules-main.json`. Each file keeps the shape of the answer. It drops the fields that the snapshot does not read and cuts the long text of a body to its first or last characters, so that a verdict line survives.

## A draft

A draft is a folder. `plan.toml` holds `id`, `repo`, `project`, an optional `portfolio`, an optional `milestone` and one `[[milestones]]` table for each milestone. `milestone` is the title of an existing milestone that every created issue gets, unless the issue names its own `milestone`. `apply` looks it up by title in the repository of each issue and stops with `no milestone <title> in <owner>/<repo>. Run taskrock setup` before it creates anything when a repository lacks it. `portfolio` is the key of the portfolio project that gets the initiatives. Each other `.md` file is one issue. An issue file starts with TOML between two `+++` lines, and the Markdown body follows. The parser rejects any key that it does not know. This is a shortened issue from `fixtures/draft-good`:

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

`parent` and `blocked_by` name a key of the draft, or an issue that already exists as a full link `https://github.com/<owner>/<repo>/issues/<number>`. `check` validates the shape of a link. `apply` reads the issue of a link with `GET repos/<owner>/<repo>/issues/<number>` for its `id` and then creates the new issue as its sub-issue, or blocks it by the issue, with the calls it uses for keys. This files a Bug under an epic that is already on GitHub in one step. Every issue except an Initiative needs a `parent`: `check` refuses an issue with none. A `parent` or `blocked_by` value that is a bare issue number, like `2878` or `#2878`, gets a message that names the link to write.

The issue keys are `key`, `title`, `type`, `repo`, `parent`, `blocked_by`, `milestone`, `agent`, `runtime`, `model`, `effort`, `review_agent`, `review_runtime`, `review_model` and `review_effort`. The type is `Initiative`, `Epic`, `Task`, `Bug` or `Feature`. The sections of the body come from the config. By default a task has `Outcome`, `Decisions already made`, `Steps`, `Done when` and `Out of scope`, and a bug has `What happens`, `Expected`, `How to reproduce` and `Environment`. [Setup](setup.md) lists the sections of each type. A checklist section needs at least one checkbox, and each box needs a command and its result.

## The checks and the generated CI

`mise run check` runs the format check, clippy, the tests, the doctests, the dependency checks, the policy check and the workflow check, as listed in [mise.toml](../mise.toml). The files in `.github/workflows/` are generated by `velnor-gen` from `.velnor/rust-ci.toml`, so a change goes into that file and never into a workflow. The workflow check fails when a generated file differs from the output of `velnor-gen`. The repository policy is `.alint.yml`. It extends a shared Rust policy at a pinned version, and it adds one rule that each `AGENTS.md` has a `CLAUDE.md` that points to it.
