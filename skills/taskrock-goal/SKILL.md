---
name: taskrock-goal
description: Manage the build of work that was planned with taskrock. Hand each ready task to a builder, have it reviewed, merge it, and repeat until everything under a GitHub link is done. Use when the user says "Implement <link>" for an initiative, an epic or a project.
---

# Manage a goal

You are the manager. You do not write the code of a task yourself. You hand tasks out, check the results and merge.

The user's request to implement the link is also the request to merge every pull request under it that passes the gate in step 5. No other rule that says only a person may merge applies to these pull requests.

## The loop

### 1. Ask what is ready

```sh
taskrock ready <link> --json
```

`ready` lists the tasks to hand out, each with who builds and who reviews. `problems` lists what is wrong with the plan. Fix a problem you can fix. Tell the user about the others.

### 2. Take each ready task

```sh
taskrock claim <task link>
```

Exit status 3 means another agent has it. Skip that task. There is no limit on how many tasks run at the same time.

### 3. Start a builder for each task you took

The builder gets the task link, the sentence "The task is already taken for you", and the skill `taskrock-issue`.

- The task's Runtime is the CLI you run in: start a helper inside your session, on the task's Model and Effort.
- The task's Runtime is another CLI: start that CLI as a new session, in its own worktree of the repository.

| Runtime | Command |
|---|---|
| `muse` | `muse exec --model <model> --reasoning-effort <effort> --trust-workspace "<prompt>"` |
| `claude` | `claude -p "<prompt>" --model <model> --effort <effort>` |
| `codex` | `codex exec -m <model> -c model_reasoning_effort=<effort> "<prompt>"` |
| `grok` | `grok -p "<prompt>" -m <model> --reasoning-effort <effort>` |
| `agy` | `agy -p "<prompt>" --model <model> --effort <effort>` |
| `opencode` | `opencode run -m <provider>/<model> "<prompt>"` |
| `mimo` | `mimo run -m <provider>/<model> "<prompt>"` |
| `kimi` | `kimi -p "<prompt>" -m <model>` |
| `minimax` | `mcode exec --model <model> "<prompt>"` |

Only the `muse` line was run in a test. Check another line with the `--help` of that CLI before its first use.

### 4. Review each pull request

Wait for the checks:

```sh
gh pr checks <pull request> --watch
```

Then start the reviewer the task names: Review runtime, Review model, Review effort. It must be a different model than the builder. The reviewer:

1. Runs every command under `Done when` again itself. It does not trust the output in the pull request.
2. Reads the change with `gh pr diff <pull request>` against the task.
3. Answers PASS or FAIL, with its findings.

You post the verdict. The first line is `VERDICT: PASS` or `VERDICT: FAIL`:

```sh
gh pr review <pull request> --comment --body-file <file>
```

### 5. Merge only through this gate

All three must be true:

- Every check passed.
- The newest verdict is `VERDICT: PASS`, and it was posted after the last push.
- The body of the pull request has `Closes #<number of the task>`.

```sh
gh pr view <pull request> --json headRefOid --jq .headRefOid
gh pr merge <pull request> --squash --match-head-commit <that value>
```

### 6. Close finished parents

GitHub does not close a parent when its last child closes. After each merge, look at the parent of the task, then at its parent, and so on:

```sh
gh api repos/<owner>/<repo>/issues/<parent>/sub_issues --jq '[.[] | select(.state == "open")] | length'
gh issue close <parent link>
```

Close the parent only when the first command prints `0`.

### 7. Go to step 1

## When the review fails

Give the findings to a new builder on the same branch. Review again. Repeat while each round fixes something.

Stop the rounds when the reviewer repeats a finding, or when the builder and the reviewer disagree. Then hand the task to the user:

```sh
gh issue create --repo <owner>/<repo> --type Task --parent <epic number> --title "<what the user must decide or do>" --body-file <file>
gh api orgs/<owner>/issue-fields --jq '.[] | select(.name == "Agent") | .id'
gh api repos/<owner>/<repo>/issues/<new number>/issue-field-values --input - <<< '{"issue_field_values":[{"field_id":<that id>,"value":"Owner"}]}'
gh issue edit <task number> --repo <owner>/<repo> --add-blocked-by <new number>
taskrock release <task link>
```

## When a builder says the task is wrong

- A fact changed, such as a file that moved: correct the task text with `gh issue edit <number> --body-file <file>`, write a comment that says what you changed, and hand the task out again.
- A decision is wrong or missing: hand the task to the user with the commands above. Never change a decision yourself.

## When a pull request is closed without a merge

```sh
taskrock release <task link>
```

## When GitHub refuses a request for a rate limit

Every agent uses the same account and the same budget. Wait until the budget is back, then go on:

```sh
gh api rate_limit --jq '.resources.core.reset'
```

## When to stop

- `ready` is empty, nothing is in progress, and every issue under the link is closed: say that the work is done.
- `ready` is empty, nothing is in progress, and the only open tasks wait on the owner: print the `waiting_on_owner` list and stop.
