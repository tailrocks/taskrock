# Design

taskrock keeps all work on GitHub and adds a small Rust tool only where `gh` is weak. A plan is written once as a draft, applied to GitHub, and then lives only there. Agents build and review the tasks. Who does what is stored in issue fields, and the owner approves the plan before anything is created.

This page keeps each choice together with the option that was rejected and the reason. The problem behind the project is in [why](why.md). The parts of the project are in [structure](structure.md). Facts that were proven by test are in [facts](facts.md).

## Principles

- GitHub is the only store. A second copy of a fact needs a sync, and a sync brings conflicts.
- The tool holds only what is hard, almost impossible or not convenient with `gh`. Everything else is plain `gh`, written in a skill.
- A task is detailed enough that a weaker model makes no design decision. Every decision is made while planning, where the strongest model and the owner are present.
- Who builds, who reviews and what blocks a task are fields and links, not text. A field can be filtered, shown in a view and checked by a rule. Text cannot.
- The author of a change does not review it. The reviewer runs on a different model than the builder.

## The model

- **Issue types.** Initiative, Epic, Task and Bug. Depth is free, up to the eight levels of GitHub. Only an issue with no children is built.
- **Routing fields.** Eight issue fields on each task. Agent, Runtime, Model and Effort say who builds. Review agent, Review runtime, Review model and Review effort say who reviews before the merge. Agent holds a role.
- **Task.** A title and five sections: Outcome, Decisions already made, Steps, Done when, Out of scope. Done when is a checklist. Each box is a real command with its expected result.
- **Owner task.** An ordinary issue with Agent set to `Owner`. It blocks the tasks that wait on it.
- **Ready task.** Open, no children, no open blocker, status "Todo", Agent not `Owner`, and no branch `feat/issue-<number>`.
- **Claim.** The branch `feat/issue-<number>` on GitHub, then the status "In Progress".
- **Giving a task back.** The status returns to "Todo" and the branch is renamed or deleted.
- **Draft.** A folder that holds a plan before it exists on GitHub. Each file is one issue. Parents and blockers name keys, so the draft is complete before any issue exists.
- **Roles.** The manager hands out tasks, starts the reviewer, posts the verdict and merges. The builder builds one task. The reviewer checks one pull request.

The [README](../README.md) lists the five commands. The three skills hold the steps that use them.

## Choices and what was rejected

### GitHub is the only store of a plan

**Chosen.** Issues, sub-issues, blockers, milestones and projects hold the work. There are no permanent plan files and no board repository.

**Rejected.** A permanent two-way mirror in a board repository. Plan files as the master with a one-way push. An MCP server that holds state in memory. A sync that stores the GitHub IDs and runs both ways.

**Reason.** The mirror keeps six copies of each fact and has about ten kinds of conflict. GitHub as the only store keeps one copy. Edits on GitHub are real. In one repository of a tracker driven by a file sync tool, 56 of its 65 issues had an edited body and 42 of them five or more times. Of the 100 newest issues in each of the two other trackers, 2 and 5 had an edited body. An edit on GitHub is an edit to the only copy. Two sync runs at once also made a duplicate issue in one tracker.

### A small custom tool, and plain `gh` for the rest

**Chosen.** Five commands: `check`, `apply`, `ready`, `claim`, `release`. Each does what `gh` cannot do or does badly.

**Rejected.** No custom command, with every operation in `gh`. A large tool with commands such as `show`, `set`, `tree`, `waiting` and `tidy`. Five commands are enough, because `gh` does the rest.

**Reason.** `gh` has no flag for issue fields, and it cannot select children of children. It cannot check a plan. GitHub accepts a blocker cycle of three issues, and the draft check finds it. A claim needs its calls in a fixed order and an undo. A plan needs a rerun that creates nothing twice. Reading routing fields by name and writing them with their IDs works with one REST call, so `show`, `set` and account setup became `gh` commands. Commands for blockers and for moving became `gh issue edit`. The tool calls `gh` for every request, so there is one login and one client.

### The tool uses the REST API, not GraphQL and not search

**Chosen.** One REST call lists a project's items with status, parent, type, open blockers, children and routing fields. One call creates an issue with type, parent and field values. A field is given by its ID and a single-select value by the option name.

**Rejected.** GraphQL for the tool, with paging. The search API for listing and for finding issues.

**Reason.** Three agents used up the hourly GraphQL budget of the account in one hour. Search lags behind writes. A new issue shows in a listing 6 to 8 seconds after it is created, so `apply` also looks for an issue by its number when the listing lags. The REST list of project items is correct right after a write. See [facts](facts.md). The tool makes one GraphQL call, `project_of`, to find the project of an issue. The one-time account setup script also uses GraphQL, to create issue types and fields.

### A plan is a draft that is applied and then used up

**Chosen.** `apply` creates every issue from a draft folder. `apply` does not read a draft again after it succeeds, and the plan skill deletes the folder. Each created issue ends with a hidden marker `<!-- taskrock:<plan>:<key> -->`. A rerun finds the marked issues in the repository listing, so it creates nothing twice, and it reads the blockers that exist before it adds one. Blockers are added in a second pass.

**Rejected.** Files that stay in sync with GitHub. A draft kept in the repository. Finding created issues by search. YAML front matter.

**Reason.** A kept draft becomes a second copy that goes stale. The marker replaces a stored map of IDs, and a listing is not search, which lags. The front matter is TOML between `+++` lines because the `toml` crate is already used in the tailrocks Rust tools and this avoids a YAML dependency. A draft is checked offline for blocker cycles, valid routing, required sections, body size and the limits of GitHub: 100 children for a parent and eight levels.

### Facts live in native GitHub features

**Chosen.** Kind of work is the issue type. Breakdown is sub-issues. Order is blocked-by links. Routing is issue fields. Status is the project's Status.

**Rejected.** `type:` and `level:` labels. A `blocked` label and "Depends on" lines in the body. `in-progress` and `status:` labels. Phase and Stage project fields that copy milestones.

**Reason.** Each fact is then stored once, on the issue. The old way drifted. In one tracker 101 closed issues still carried `in-progress`. A phase is an epic. A milestone is optional. A draft can name one for a group of tasks.

### Routing is eight issue fields

**Chosen.** The eight fields of the model, written when the plan is created. The planner fills them, by default with a reviewer on a different model.

**Rejected.** `model:`, `exec:` and `effort:` labels. Project fields. Routing as text in the body.

**Reason.** The three existing trackers recorded the model as a label, as a project field, as body text, or as a mix of these. An issue field is native, is set in the same call that creates the issue, and is read by name through REST. GitHub's default field `Effort` means work size, so an account renames it to `Size` before it gets the routing field `Effort`.

### Every issue is in a project, and the project is private

**Chosen.** Every issue the tool manages belongs to one project. The status is the Status field of that project.

**Rejected.** Small work as an issue with no project. A public project.

**Reason.** Status needs a project, so an issue without one has no status. GitHub does not allow these fields in a public project. A new project is a copy of a template project, because `gh project copy` keeps the views, the fields and the workflows. The tool has no command that sets up a project.

### Work for the owner is an ordinary issue

**Chosen.** An issue with Agent `Owner` blocks the tasks that wait on it. The ready list leaves it out. The "Needs you" view lists every open issue that waits on the owner. A decision that can be settled while planning is settled there and never becomes an issue.

**Rejected.** The issue types Decision and Owner action. An `owner-action` label.

**Reason.** A task that waits on an owner issue shows GitHub's own blocked mark, and one view lists the owner issues, so no extra type is needed. All agents act as the owner's account, and GitHub sends no notification for a person's own actions. So the owner learns that something waits from the view and from the paused goal session.

### A branch on GitHub is the claim

**Chosen.** `taskrock claim` creates the branch `feat/issue-<number>`. Only then does it set the status. A task can be taken only when it is open, has no children, has status "Todo", has no open blocker and is not an owner task.

**Rejected.** An assignee. A "taken by" comment. The status alone, which cannot be the claim because a status check and a write can race.

**Reason.** GitHub lets one creator of a branch name succeed. Of four attempts in the same moment, one succeeded and three were refused. Every sampled edit and comment in the three trackers was made by one login, so GitHub cannot tell two agents apart and an assignee alone is not a claim. GitHub has no "update only if unchanged" write, so a status check is a read followed by a write, and several goals run at once. `gh issue develop` and the GraphQL linked-branch call do not refuse a second claim. Only the REST branch call does. If the status write fails, `claim` deletes its branch. Without that, a task looks ready and cannot be taken. `ready` leaves out a task whose branch exists.

### Taking a task changes the status and nothing else

**Chosen.** Taking sets the status to "In Progress" and does the same for every open parent up to the initiative. Giving back sets "Todo". A branch with commits is renamed, so the work is kept. An empty branch is deleted. A parent returns to "Todo" when no child is in progress or done.

**Rejected.** A comment or an assignee that records who took the task. Setting no parent status.

**Reason.** The board then shows which epics are being worked on. Deleting a branch that holds work would lose it. GitHub does not close a parent when its last child closes, so the manager closes a parent after each merge when all its children are closed.

### The review happens on the pull request, as a comment

**Chosen.** The reviewer runs every `Done when` command again itself, then reads the change. The manager posts the result as a comment that starts with `VERDICT: PASS` or `VERDICT: FAIL`.

**Rejected.** An approving review. A separate verifier role. A review as an issue or a sub-issue of the task.

**Reason.** All agents share one GitHub account and one account cannot approve its own pull request. Correctness is checked three times: the builder proves each command, the tests run on GitHub, and the reviewer runs the commands again and never trusts pasted output, so a verifier is not needed. A blocker cannot say "after the pull request opens and before it merges". Review issues would also at least double the number of issues. The pull request is where GitHub keeps reviews. A larger review, such as a security review of an epic, is an ordinary task blocked by the tasks it covers.

### The manager merges through a fixed gate, by squash

**Chosen.** Merge only when the checks pass, the newest verdict is PASS, and the pull request says `Closes #<number>`. The owner's request to implement a link is also the request to merge. Merges are squash merges.

**Rejected.** Merging when the review is only a comment, with no checks and no gate.

**Reason.** A review comment alone does not stop a merge, so the gate requires that the newest verdict is PASS. The gate is written into the goal skill. The verdict must be posted after the last push, and the merge names the head commit it checked. Squash merges match the repositories of the owner, which allow only squash.

### A builder that finds the task wrong stops

**Chosen.** The builder writes what it found on the issue, gives the task back and stops. The manager corrects a changed fact, such as a moved file, and hands the task out again. A changed decision becomes an owner issue that blocks the task.

**Rejected.** No other option was recorded.

**Reason.** The builder makes no design decision, so a wrong task is a planning error and only a decision-maker can fix it.

### No fixed limit on builders and on fix rounds

**Chosen.** Any number of builders and goals run at once. Review and fix repeat while each round fixes something. The cycle ends when the reviewer repeats a finding or the builder and the reviewer disagree. The task then gets an owner issue that blocks it, and is given back.

**Rejected.** A cap on builders. A fixed number of fix rounds. Review and fix with no end.

**Reason.** Several goals may run at once, as a team of agents, so a cap does not fit. Two tasks that change the same file get a blocker, which is what keeps them from conflicting. On a rate limit the manager waits for the reset, because all agents share one request budget. A round that fixes nothing new means the cycle must end, so it cannot loop forever.

### A task is one session, one branch and one pull request

**Chosen.** A task is one session of work, one branch and one pull request. It is a narrow path through every layer that can be proven alone.

**Rejected.** No other option was recorded.

### The planner asks only when a guess would change the breakdown

**Chosen.** It asks when a wrong guess changes the breakdown and nothing settles it: not the owner's words, the code or the docs. Everything else becomes an assumption beside the breakdown. Questions come once, in one batch, each with a suggested answer. The owner approves a short tree and the full text of a task is shown on request.

**Rejected.** A fixed round of questions before every plan. Questions about things the owner already explained.

**Reason.** The owner does not want to be asked what was already said. A listed assumption can be corrected in a word at approval. Nothing is created before the approval.

### One crate

**Chosen.** One Rust crate. The library holds the logic and `main.rs` parses arguments. Tests use a fake `gh`.

**Rejected.** Three crates: a pure core, a GitHub boundary and a thin command-line crate.

**Reason.** Five commands do not need three crates. The layers are modules of one crate instead. See [structure](structure.md).

### The three skills are self-contained

**Chosen.** Three skills that hold the whole path and call no skill of another package.

**Rejected.** Handing the build to the playbooks of pstack. A fork of pstack. A separate grilling skill for the question round. The pull request skill of another package.

**Reason.** The skills name the exact `taskrock` and `gh` commands of each step, so a session needs no other plugin to plan, build and review. See [comparison](comparison.md).

### Skills ship as a native plugin package

**Chosen.** The three skills are one plugin package, installed through the plugin system of each agent CLI. Package version and tool version change together.

**Rejected.** Skill folders copied by hand into a user folder, with the skills kept in the tool repository.

**Reason.** A skill must never name a command the installed tool lacks, and a different tool version can change the commands. The package version matches the tool version. A skill also needs its sibling skills and references in their original places, so the package is installed as one unit and a single `SKILL.md` is not copied out.

## Left out of version 1 on purpose

- **The tool starting other agent CLIs.** The goal skill lists the launch command for each CLI instead. Only the Muse Code line was run in a test. Starting a second CLI from inside a session was not verified.
- **A report of broken rules on GitHub.** It would report and not fix. It is left out with its scheduled workflow, because the tool keeps only what `gh` cannot do.
- **Resetting a task whose builder died.** One idea was to count a task as abandoned after two hours with no push and no pull request. `taskrock release` does it by hand.
- **Moving the existing trackers to this setup.** The tool does not do it.
- **Projects owned by a personal account.** The tool reads and writes projects of an organization.
- **A separate GitHub identity for agents.** It would let GitHub notify the owner. It is not part of version 1.
- **Also left out.** A steering issue for corrections, export and rewrite of issues, bulk changes by query, `tree` and `waiting` commands, a setup skill and an install command.
