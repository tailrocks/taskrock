# Design

taskrock keeps all work on GitHub and adds a small Rust tool only where `gh` is weak. A plan is written once as a draft, applied to GitHub, and then lives only there. Agents build and review the tasks. Who does what is stored in issue fields, and the owner approves the plan before anything is created.

This page keeps each choice together with the option that was rejected and the reason. The problem behind the project is in [why](why.md). The parts of the project are in [structure](structure.md). The setup of an organization, its config file and its issue formats are in [setup](setup.md). Facts that were proven by test are in [facts](facts.md).

## Principles

- GitHub is the only store. A second copy of a fact needs a sync, and a sync brings conflicts.
- The tool holds only what is hard, almost impossible or not convenient with `gh`. Everything else is plain `gh`, written in a skill.
- A task is detailed enough that a weaker model makes no design decision. Every decision is made while planning, where the strongest model and the owner are present.
- Who builds, who reviews and what blocks a task are fields and links, not text. A field can be filtered, shown in a view and checked by a rule. Text cannot.
- The author of a change does not review it. The reviewer runs on a different model than the builder.

## The model

- **Issue types.** Initiative, Epic, Task, Bug and Feature. Depth is free, up to the eight levels of GitHub. Only an issue with no children is built. A Bug or Feature is a report: it starts in Triage and is built after triage.
- **Status.** Triage, Todo, In Progress and Done. Reported issues start in Triage, planned issues in Todo.
- **Routing fields.** Eight issue fields on each task. Agent, Runtime, Model and Effort say who builds. Review agent, Review runtime, Review model and Review effort say who reviews before the merge. Agent holds a subagent name from the config.
- **Task.** A title and five sections: Outcome, Decisions already made, Steps, Done when, Out of scope. Done when is a checklist. Each box is a real command with its expected result. Each type has its own sections, set in the config file of the organization.
- **Config.** One file for each organization, `<owner>/.github/taskrock.toml`. It holds the routing blocks, the issue formats, the projects with their repositories, workflows and shared milestones. The views of each kind of project are fixed by the tool. `taskrock setup` makes GitHub match it.
- **Owner task.** An ordinary issue with Agent set to `Owner`. It blocks the tasks that wait on it.
- **Ready task.** Open, no children, no open blocker, status "Todo", Agent not `Owner`, a body that has the build sections of its type, and no branch `feat/issue-<number>`. With `--stacked`, `ready` also lists a task whose open blockers form one stack: each is In Progress with an open pull request from its branch. The entry names the branch to start from.
- **Claim.** The branch `feat/issue-<number>` on GitHub, then the status "In Progress". `claim --stacked` creates the branch from the branch of the top blocker instead of the default branch, and records that base in a comment on the task.
- **Giving a task back.** The status returns to "Todo" and the branch is renamed or deleted. A handoff note can be posted first.
- **Draft.** A folder that holds a plan before it exists on GitHub. Each file is one issue. Parents and blockers name keys, so the draft is complete before any issue exists. A parent or blocker may also be a full link to an issue that already exists, and `plan.toml` may name a milestone for every issue it creates. This files an issue inside a goal in one step.
- **Roles.** The manager hands out tasks, starts the reviewer, posts the verdict and merges. The builder builds one task. The reviewer checks one pull request.

The [README](../README.md) does not yet list the three `pr` commands. The skills hold the steps that use them.

## Choices and what was rejected

### GitHub is the only store of a plan

**Chosen.** Issues, sub-issues, blockers, milestones and projects hold the work. There are no permanent plan files and no board repository.

**Rejected.** A permanent two-way mirror in a board repository. Plan files as the master with a one-way push. An MCP server that holds state in memory. A sync that stores the GitHub IDs and runs both ways.

**Reason.** The mirror keeps six copies of each fact and has about ten kinds of conflict. GitHub as the only store keeps one copy. Edits on GitHub are real. In one repository of a tracker driven by a file sync tool, 56 of its 65 issues had an edited body and 42 of them five or more times. Of the 100 newest issues in each of the two other trackers, 2 and 5 had an edited body. An edit on GitHub is an edit to the only copy. Two sync runs at once also made a duplicate issue in one tracker.

### A small custom tool, and plain `gh` for the rest

**Chosen.** Eleven commands: `check`, `apply`, `ready`, `claim`, `release`, `setup`, `template`, `edit`, `pr status`, `pr threads` and `pr merge`. Each does what `gh` cannot do or does badly.

**Rejected.** No custom command, with every operation in `gh`. A large tool with commands such as `show`, `set`, `tree`, `waiting` and `tidy`. Eleven commands are enough, because `gh` does the rest.

**Reason.** `gh` has no flag for issue fields, and it cannot select children of children. It cannot check a plan. GitHub accepts a blocker cycle of three issues, and the draft check finds it. A claim needs its calls in a fixed order and an undo. A plan needs a rerun that creates nothing twice. Reading routing fields by name and writing them with their IDs works with one REST call, so `show` and `set` became `gh` commands. Account setup became `taskrock setup`, because the options of an issue field and the views of a project need calls in a fixed order that a script got wrong. Commands for blockers and for moving became `gh issue edit`. The tool calls `gh` for every request, so there is one login and one client.

### The tool uses the REST API, not GraphQL and not search

**Chosen.** One REST call lists a project's items with status, parent, type, open blockers, children and routing fields. One call creates an issue with type, parent and field values. A field is given by its ID and a single-select value by the option name.

**Rejected.** GraphQL for the tool, with paging. The search API for listing and for finding issues.

**Reason.** Three agents used up the hourly GraphQL budget of the account in one hour. Search lags behind writes. A new issue shows in a listing 6 to 8 seconds after it is created, so `apply` also looks for an issue by its number when the listing lags. The REST list of project items is correct right after a write. See [facts](facts.md). The tool reads GraphQL in a few places. `project_of` finds the project of an issue, for `claim`, `release`, `ready` and `edit`. `projects::kind_of` reads the kind of a project for `ready` with a project link, and `projects::list` lists the projects of an organization for `apply` with a portfolio key. `taskrock setup` also uses GraphQL, because projects, views and workflows have no REST endpoint.

### A plan is a draft that is applied and then used up

**Chosen.** `apply` creates every issue from a draft folder. `apply` does not read a draft again after it succeeds, and the plan skill deletes the folder. Each created issue ends with a hidden marker `<!-- taskrock:<plan>:<key> -->`. A rerun finds the marked issues in the repository listing, so it creates nothing twice, and it reads the blockers that exist before it adds one. Blockers are added in a second pass.

**Rejected.** Files that stay in sync with GitHub. A draft kept in the repository. Finding created issues by search. YAML front matter.

**Reason.** A kept draft becomes a second copy that goes stale. The marker replaces a stored map of IDs, and a listing is not search, which lags. The front matter is TOML between `+++` lines because the `toml` crate is already used in the tailrocks Rust tools and this avoids a YAML dependency. A draft is checked offline for blocker cycles, valid routing, required sections, body size and the limits of GitHub: 100 children for a parent and eight levels.

### An issue filed while a goal is built goes under the goal

**Chosen.** A Bug, a follow-up task, a task for the owner or a retro that is filed while a goal is built comes from a one-file draft. Its `parent` is a link under the goal, and `plan.toml` gives it the goal's milestone. `ready` lists the Triage items under the link. The goal skill triages them, builds them, and stops only when the goal's milestone has no open issue.

**Rejected.** Friction Bugs with no parent and no milestone that wait in Triage outside the goal. A separate `gh issue edit --parent` call after `apply`.

**Reason.** In the 0.2.0 build, six Bugs that reviewers filed stayed in Triage with no parent. `ready` with the initiative link never saw them, and the initiative closed as done. A second call after `apply` is a step that an agent skips.

### Nothing is built before a claim

**Chosen.** A builder writes code only for an issue that `taskrock claim` took: Status In Progress and the branch `feat/issue-<number>` on GitHub. A blocked task waits until its blockers close, or until the manager takes it with `taskrock claim --stacked`. An issue in Triage is built only after `taskrock-triage` set it to Todo.

**Rejected.** Building a blocked task early on the branch of its blocker without a claim. A `claim --early` that takes a task whose blocker has no open pull request. Building fixes in local worktrees before the issue is claimed.

**Reason.** The Status is the only record of the work. Work that GitHub does not show is invisible to the other agents and to the owner. A stacked task is claimed like any other task, so GitHub shows it. Version 0.2.1 rejected stacked pull requests. The owner reversed that in [#139](https://github.com/tailrocks/taskrock/issues/139) to finish plans faster. See the next section.

### A stacked task starts from its blocker's branch

**Chosen.** `taskrock claim --stacked` takes a Todo task whose open blockers form one stack. Each open blocker is in the same repository, is In Progress, and has an open pull request from `feat/issue-<blocker>`. With several open blockers, one of them must have all the others below it, following the base branches of the pull requests. The task's branch starts from the branch of that top blocker. The goal skill merges a pull request only when its base is the default branch. Before it merges one, it retargets every open pull request whose base is its branch to the default branch. The builder of each retargeted task then merges the default branch, and the task is reviewed again. A stacked task is still one branch and one pull request.

**Rejected.** Two open blockers that are not one stack: the pull request would need two bases. Merging the branches of several blockers into one base branch. Retargeting after the merge, or leaving it to GitHub. Rebasing a stacked branch with a force push.

**Reason.** The owner asked for stacked pull requests in [#139](https://github.com/tailrocks/taskrock/issues/139) on 2026-11-27, to finish plans faster. This reverses the rejection of 0.2.1. A pull request has exactly one base, so a task can start from one chain only. A plan often names a blocker twice, directly and through another blocker that is stacked on it, and the chain rule accepts that case. In a sandbox test, deleting the base branch after a squash merge closed the stacked pull request, and it cannot be reopened. The repositories delete a head branch on merge, so the goal retargets first. Merging the default branch after the base merged left only the task's own change in the diff, with no force push. See [facts](facts.md).

### Facts live in native GitHub features

**Chosen.** Kind of work is the issue type. Breakdown is sub-issues. Order is blocked-by links. Routing is issue fields. Status is the project's Status.

**Rejected.** `type:` and `level:` labels. A `blocked` label and "Depends on" lines in the body. `in-progress` and `status:` labels. Phase and Stage project fields that copy milestones.

**Reason.** Each fact is then stored once, on the issue. The old way drifted. In one tracker 101 closed issues still carried `in-progress`. A phase is an epic. A milestone is optional. A draft can name one for a group of tasks.

### Routing is eight issue fields

**Chosen.** The eight fields of the model, written when the plan is created. The planner fills them, by default with a reviewer on a different model.

**Rejected.** `model:`, `exec:` and `effort:` labels. Project fields. Routing as text in the body.

**Reason.** The three existing trackers recorded the model as a label, as a project field, as body text, or as a mix of these. An issue field is native, is set in the same call that creates the issue, and is read by name through REST. GitHub's default field `Effort` means work size, so `taskrock setup` renames it to `Size` before it creates the routing field `Effort`.

### Every issue is in a project, and the project is private

**Chosen.** Every issue the tool manages belongs to one project. The status is the Status field of that project.

**Rejected.** Small work as an issue with no project. A public project. A new project as a copy of a template project.

**Reason.** Status needs a project, so an issue without one has no status. GitHub does not allow these fields in a public project. `taskrock setup` creates and sets up every project through the API. There is no template project and no copy. See [setup](setup.md).

### Work for the owner is an ordinary issue

**Chosen.** An issue with Agent `Owner` blocks the tasks that wait on it. The ready list leaves it out. The "Needs you" view lists every open issue that waits on the owner. A decision that can be settled while planning is settled there and never becomes an issue.

**Rejected.** The issue types Decision and Owner action. An `owner-action` label.

**Reason.** A task that waits on an owner issue shows GitHub's own blocked mark, and one view lists the owner issues, so no extra type is needed. All agents act as the owner's account, and GitHub sends no notification for a person's own actions. So the owner learns that something waits from the view and from the paused goal session.

### A branch on GitHub is the claim

**Chosen.** `taskrock claim` creates the branch `feat/issue-<number>`. Only then does it set the status. A task can be taken only when it is open, has no children, has status "Todo", has no open blocker, or with `--stacked` open blockers that form one stack, and is not an owner task.

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

### The pull request gate

**Chosen.** `taskrock pr status`, `taskrock pr threads` and `taskrock pr merge`. The verdict is a `VERDICT: PASS|FAIL head=<sha>` text. A round is one reviewed head. Findings of P0, P1 and P2 block, from bots and from taskrock reviewers. A bot P0 to P2 thread blocks until the reviewer rules on it. P3 bot threads are resolved with a reply. Codex reviews a ready pull request on every push and skips drafts, so a builder works in draft and marks the pull request ready once per round. No follow-up issues for review findings.

**Rejected.** GitHub approvals (one shared login). Counting only FAIL verdicts. Merging known medium findings. Turning Codex automatic reviews off. Follow-up issues per finding. A `pr refresh` command and force-push rebases. Refreshing a pull request that is only behind.

**Reason.** A velnor-new pull request reached 14 FAIL verdicts and 19 bot reviews. Every push started a new review. A refresh is a push, and a push is a new round.

### A builder that finds the task wrong stops

**Chosen.** The builder writes what it found on the issue, gives the task back and stops. The manager corrects a changed fact, such as a moved file, and hands the task out again. A changed decision becomes an owner issue that blocks the task.

**Rejected.** No other option was recorded.

**Reason.** The builder makes no design decision, so a wrong task is a planning error and only a decision-maker can fix it.

### No limit on builders, and a cap on fix rounds

**Chosen.** Any number of builders and goals run at once. Fix rounds are capped by `review.max_rounds` in the config, 3 by default. After that many reviewed heads, or when the reviewer repeats a finding or the builder and the reviewer disagree, the task gets an owner issue that blocks it, and is given back.

**Rejected.** A cap on builders. No cap on fix rounds. Review and fix with no end.

**Reason.** Several goals may run at once, as a team of agents, so a cap on builders does not fit. Two tasks that change the same file get a blocker, which is what keeps them from conflicting. `ready` also warns when two ready tasks name the same file. On a rate limit the manager waits for the reset, because all agents share one request budget. A round that fixes nothing new means the cycle must end, and a cap makes sure it ends even when each round finds something new.

### A task is one session, one branch and one pull request

**Chosen.** A task is one session of work, one branch and one pull request. It is a narrow path through every layer that can be proven alone. A stacked task is also one branch and one pull request. Only its base differs until its blocker merges.

**Rejected.** No other option was recorded.

### The planner asks only when a guess would change the breakdown

**Chosen.** It asks when a wrong guess changes the breakdown and nothing settles it: not the owner's words, the code or the docs. Everything else becomes an assumption beside the breakdown. Questions come once, in one batch, each with a suggested answer. The owner approves a short tree and the full text of a task is shown on request.

**Rejected.** A fixed round of questions before every plan. Questions about things the owner already explained.

**Reason.** The owner does not want to be asked what was already said. A listed assumption can be corrected in a word at approval. Nothing is created before the approval.

### One crate

**Chosen.** One Rust crate. The library holds the logic and `main.rs` parses arguments. Tests use a fake `gh`.

**Rejected.** Three crates: a pure core, a GitHub boundary and a thin command-line crate.

**Reason.** Eleven commands do not need three crates. The layers are modules of one crate instead. See [structure](structure.md).

### The skills are self-contained

**Chosen.** Skills that hold the whole path and call no skill of another package.

**Rejected.** Handing the build to the playbooks of pstack. A fork of pstack. A separate grilling skill for the question round. The pull request skill of another package.

**Reason.** The skills name the exact `taskrock` and `gh` commands of each step, so a session needs no other plugin to plan, build and review. See [comparison](comparison.md).

### Skills ship as a plugin in their own marketplace repository

**Chosen.** The skills are the plugin `taskrock` in `tailrocks/taskrock-plugins`. That repository is also the marketplace: its catalog files name the plugin by the local path `./plugins/taskrock`. One skill tree serves five agent CLIs, Claude Code, Codex, Grok Build, Muse Code and Antigravity, each through its own manifest. Every skill first runs `taskrock --version` and stops when the tool is missing or older than it needs. The CI of the plugin installs the released tool and runs `taskrock <command> --help` for each command a skill names.

**Rejected.** Skill folders copied by hand into a user folder. A plugin package in its own repository that a shared Tailrocks catalog pins to a commit, which was the layout of 0.1.0. The skills inside the tool repository. A catalog that points into the tool repository with `git-subdir`.

**Reason.** A skill must never name a command the installed tool lacks. The version check and the CI check hold that rule across two repositories, and the release order is the tool first, then the plugin. The 0.1.0 layout needed three repositories for one tool, put tailrocks roles and a tailrocks template project into the skills, and installed a 182 MB `node_modules` for an OpenCode adapter into the cache of Claude Code. Adding a marketplace clones its whole repository on every agent, so the tool repository would give each agent the Rust source too. Claude Code reads `git-subdir`, but no test showed that the other four agents do. A local path in the plugin repository works on all five. A skill also needs its sibling skills in their original places, so the plugin is installed as one unit and a single `SKILL.md` is not copied out.

## Left out of version 1 on purpose

- **The tool starting other agent CLIs.** The goal skill lists the launch command for each CLI instead. Only the Muse Code line was run in a test. Starting a second CLI from inside a session was not verified.
- **A report of broken rules on GitHub.** It would report and not fix. It is left out with its scheduled workflow, because the tool keeps only what `gh` cannot do.
- **Resetting a task whose builder died.** One idea was to count a task as abandoned after two hours with no push and no pull request. `taskrock release` does it by hand.
- **Moving the existing trackers to this setup.** The tool does not do it.
- **Projects owned by a personal account.** The tool reads and writes projects of an organization.
- **A separate GitHub identity for agents.** It would let GitHub notify the owner. It is not part of version 1.
- **Also left out.** A steering issue for corrections, export and rewrite of issues, bulk changes by query, and `tree` and `waiting` commands.
- **What `setup` does not do.** It never deletes a type, field, option, view, link, milestone or project, and it does not copy a project. [Setup](setup.md) lists them.
