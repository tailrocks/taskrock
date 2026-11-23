---
name: taskrock-issue
description: Work on one GitHub issue that was planned with taskrock. Read the task, take it, build it by its Steps, prove it by its Done when, and open the pull request. Use when the user says "Do <issue link>" or when a manager hands you one task.
---

# Work on one issue

You get one issue link. The issue is the whole task. Build only what it says.

## 1. Read

```sh
gh issue view <link>
gh api repos/<owner>/<repo>/issues/<number> --jq '[.issue_field_values[] | "\(.issue_field_name)=\(.single_select_option.name)"] | join("; ")'
```

The second command prints who builds and who reviews. Read `AGENTS.md` in the repository too.

## 2. Take the task

Skip this step when the manager told you that the task is already taken for you.

```sh
taskrock claim <link>
```

- Exit status 0: the task is yours. Its branch `feat/issue-<number>` exists on GitHub.
- Exit status 3: the answer is no. Another agent has it, or it is not ready. Stop and say so.

## 3. Switch to the branch

```sh
git fetch origin feat/issue-<number>
git switch feat/issue-<number>
```

When other agents work in the same checkout, use your own folder:

```sh
git worktree add ../<repo>-issue-<number> feat/issue-<number>
```

Never create another branch.

## 4. Build

- Follow `Steps` in order.
- Use the names in `Decisions already made` exactly: files, types, functions, commands, messages.
- Make no design decision.
- Stay inside `Out of scope`.
- Commit often, with `git commit -s`. Push often.

## 5. Prove

Run every command under `Done when`. Keep each command and the output you saw. A box is done only after you saw the result yourself.

## 6. Open the pull request

Write the body to a file. It has four sections and one line:

```markdown
## Why
## What
## Proof
## Done when

Closes #<number>
```

`Proof` holds each `Done when` command with its output. `Done when` repeats the checklist with every box ticked.

```sh
GH_PR_CREATE_DRAFT=0 gh pr create --base <default branch> --head feat/issue-<number> --title "<type>: <summary>" --body-file <file>
```

The title follows Conventional Commits. Then report the link of the pull request and stop. Do not merge.

## When the task is wrong

The task can name a file that moved, or a step that no longer fits. Do not guess and do not push on.

```sh
gh issue comment <link> --body-file <file>
taskrock release <link>
```

Write in the comment what you found and why you cannot go on. Then stop.
