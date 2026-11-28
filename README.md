# taskrock

GitHub is the only place work lives: issues, projects and pull requests. `taskrock` holds only what is hard, almost impossible or not convenient with the `gh` command.

## Install

```toml
# mise.toml
[tools]
"cargo:https://github.com/tailrocks/taskrock" = { version = "tag:v0.5.2", crate = "taskrock" }
```

The tool needs Rust 1.100.0 or newer.

The skills are the plugin `taskrock` in [tailrocks/taskrock-plugins](https://github.com/tailrocks/taskrock-plugins). That repository is its own marketplace for Claude Code, Codex, Grok Build, Muse Code and Antigravity. Its [installation guide](https://github.com/tailrocks/taskrock-plugins/blob/main/docs/installation.md) has the commands for each agent. For Claude Code:

```sh
claude plugin marketplace add tailrocks/taskrock-plugins
claude plugin install taskrock@taskrock --scope user
```

Install the skills only as a plugin. Do not copy skill folders by hand. Remove the old plugin `tailrocks-taskrock-skills` and old copies named `taskrock-*` from `~/.agents/skills` and `~/.claude/skills`.

## Set up an account

Run it once for each organization. The setup reads one file, `taskrock.toml`, that describes how the organization works. It creates only what is missing, so it is safe to run again.

A minimal `taskrock.toml`, saved in the repository `<owner>/.github`. It lists the five required issue types, here with the default formats:

```toml
version = 1

[routing]
runtimes = ["claude"]

[[routing.subagent]]
name     = "builder"
runtimes = ["claude"]
can      = ["build", "review"]

[[routing.model]]
name     = "claude-sonnet-5-5"
runtimes = ["claude"]
efforts  = ["low", "medium", "high"]
use      = "The default builder."

[[issue_type]]
name      = "Initiative"
file      = ["Outcome", "Why", "Not yet specified", "Out of scope", "Done when"]
build     = ["Outcome", "Why", "Not yet specified", "Out of scope", "Done when"]
checklist = ["Done when"]

[[issue_type]]
name      = "Epic"
file      = ["Outcome", "Decisions already made", "Not yet specified", "Out of scope", "Done when"]
build     = ["Outcome", "Decisions already made", "Not yet specified", "Out of scope", "Done when"]
checklist = ["Done when"]

[[issue_type]]
name      = "Task"
file      = ["Outcome", "Decisions already made", "Steps", "Done when", "Out of scope"]
build     = ["Outcome", "Decisions already made", "Steps", "Done when", "Out of scope"]
checklist = ["Done when"]

[[issue_type]]
name      = "Bug"
file      = ["What happens", "Expected", "How to reproduce", "Environment"]
build     = ["What happens", "Expected", "How to reproduce", "Environment", "Cause",
             "Decisions already made", "Steps", "Done when", "Out of scope"]
checklist = ["Done when"]

[[issue_type]]
name      = "Feature"
file      = ["Problem", "Proposed behavior", "Workaround today"]
build     = ["Problem", "Proposed behavior", "Workaround today",
             "Decisions already made", "Steps", "Done when", "Out of scope"]
checklist = ["Done when"]

[[project]]
key   = "main"
title = "Main"
kind  = "delivery"
repos = ["my-repo"]
```

Check the file, see what would change, then make the change:

```sh
taskrock check taskrock.toml
taskrock setup <owner> --dry-run
taskrock setup <owner>
```

`taskrock setup <owner> --file taskrock.toml` reads a local copy. [docs/setup.md](docs/setup.md) describes the file, the issue types, the kinds of project and each step of the setup.

## Use

The owner works in three ways, each through a skill of the plugin `taskrock`. Always name the skill, so that the agent does not pick another one:

| Agent | Plan | Build one task | Build an initiative, epic or project |
| --- | --- | --- | --- |
| Claude Code | `/taskrock:taskrock-plan` | `/taskrock:taskrock-issue <link>` | `/taskrock:taskrock-goal <link>`, or `/goal /taskrock:taskrock-goal <link>` |
| Codex | `$taskrock:taskrock-plan` | `$taskrock:taskrock-issue <link>` | `$taskrock:taskrock-goal <link>` |
| Grok Build, Muse Code, Antigravity | `/taskrock-plan` | `/taskrock-issue <link>` | `/taskrock-goal <link>` |
| Any agent | "use the skill taskrock-plan" | "use the skill taskrock-issue with `<link>`" | "use the skill taskrock-goal with `<link>`" |

## Commands

| Command | Description |
| --- | --- |
| `check` | Check a draft or a `taskrock.toml` file against the rules. Uses no network. `--config <file>` reads the issue formats from a config file. `--coverage` prints a read-only report on a draft |
| `apply` | Create the plan of a draft on GitHub. `parent` and `blocked_by` in a draft may be a full issue link to an issue that already exists, so one step files a Bug under an existing epic. `milestone` in `plan.toml` sets a milestone on every created issue |
| `ready` | List the tasks that can start now, and the issues waiting in Triage. `--json` prints them as a `triage` list. With an issue link it also lists the open issues of the link's milestone that have no parent, under `LOOSE` and as a `loose` list. `--stacked` also lists a task whose blockers' pull requests are open, with the branch to start from |
| `claim` | Take a task. `--stacked` starts it from the branch of its blocker's open pull request |
| `release` | Give a task back. `--note <file>` posts a handoff comment first |
| `setup` | Make GitHub match the `taskrock.toml` of an organization. `--file <path>` reads a local copy, `--dry-run` changes nothing |
| `template` | Print an empty body with the file sections of an issue type |
| `edit` | Replace the body of an issue after checking it against its type. `--replan` allows a change to the frozen sections |

## Documentation

The folder [`docs/`](docs/README.md) explains [why taskrock exists](docs/why.md), its [design](docs/design.md) with the options that were rejected, the [setup](docs/setup.md) of an organization, its [structure](docs/structure.md), a [comparison](docs/comparison.md) with the projects that were studied, and the [facts proven by test](docs/facts.md).

## Develop

```sh
mise install
mise run check
```

## License

Apache-2.0. See [LICENSE](LICENSE).
