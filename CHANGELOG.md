# Changelog

All notable changes to this project are recorded in this file.

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
