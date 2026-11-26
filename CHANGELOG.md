# Changelog

All notable changes to this project are recorded in this file.

## Unreleased

- The skills moved to the plugin `taskrock` in `tailrocks/taskrock-plugins`, which is its own marketplace for Claude Code, Codex, Grok Build, Muse Code and Antigravity. The package `tailrocks/tailrocks-taskrock-skills` is replaced.

## 0.1.0

- `check`: check a draft against the rules, using no network.
- `apply`: create the plan of a draft on GitHub.
- `ready`: list the tasks that can start now.
- `claim`: take a task.
- `release`: give a task back.
- The `taskrock-goal`, `taskrock-issue` and `taskrock-plan` skills.
