# Setup

This page describes version 0.4.0.

taskrock is one system of three parts: the skills, the `taskrock` command and plain `gh`. Version 0.2.0 adds what makes it complete for an organization:

1. **One config file** for each organization, `<owner>/.github/taskrock.toml`. It describes how the organization works: subagents, runtimes, models, issue types with their formats, and projects with their repositories, workflows and shared milestones. The views of each kind of project are fixed by the tool.
2. **`taskrock setup <owner>`**. It makes GitHub match the file. It needs no browser and copies no project.
3. **An issue model** with five types, a Triage status and a format for each type that the command enforces.
4. **Skills** that read the config.

Everything is done by the command, `gh` and the skills. Nobody opens the browser. The choices behind this page are in [design](design.md). The parts of the code are in [structure](structure.md). The facts about GitHub that setup stands on are in [facts](facts.md), and this page names them F1 to F11.

## 1. How GitHub organizes work

| Part | Belongs to | Spans repositories | Set by API | taskrock uses it for |
|---|---|---|---|---|
| Issue | a repository | its sub-issues and blockers can be in other repositories | yes | the unit of work |
| Issue type | the organization (25 at most) | yes | yes | Initiative, Epic, Task, Bug, Feature |
| Issue field | the organization (25 at most) | yes, one value seen in every project | yes, options only by REST with IDs (F5) | the eight routing fields |
| Project | the organization | yes | yes | Status, views, the Ready list |
| Project field | one project | no | yes | Status only |
| View | one project | | name, layout, filter, columns (F1) | the views of each kind |
| View group-by, sort, tree nesting, roadmap dates | one view | | **no** (F2, F9) | defaults are used |
| Project workflow | one project | | delete only (F11) | the six defaults stay on |
| Milestone | a repository | no: a shared release is the same title in each repository | yes | releases |
| Label | a repository | no | yes | not used |

Three consequences:

1. **A project is a view over issues, not a container.** An issue can be an item of many projects.
2. **Issue types and issue fields are shared by every project. Project fields are not.** A fact that more than one project needs is an issue field.
3. **Status is a project field.** An issue in two projects has two statuses. Exactly one project owns the status of each built issue.

## 2. Kinds of project

| Option | Rejected because |
|---|---|
| One project for the organization | unrelated products share one board, and the filters grow with every product |
| One project for each plan (as in 0.1.0) | a project is used up after one initiative, Ready lists are split, and views are copied each time |
| One project for each repository | a product with several repositories is split, and its blockers cross boards |
| One project for each team | all agents share one login |

**Chosen: delivery projects and portfolio projects.**

- **Delivery project.** One product or long-lived stream of work over all its repositories, for example `taskrock` over taskrock, taskrock-plugins and taskrock-sandbox. Plans are applied to it. `ready`, `claim` and `release` read and set its Status.
- **Portfolio project.** An overview across products, for example `Roadmap`. It holds initiatives. taskrock never reads or sets its Status. Dates and progress come from issue fields and `Sub-issues progress`, which are the same in every project.

Rules:

1. Each Task, Bug and Feature is an item of exactly one delivery project.
2. Initiatives and epics are items of the delivery project of their children. Each initiative is also an item of the portfolio project that `plan.toml` names.
3. A repository can be linked to several delivery projects. A plan names its project.
4. A plan never creates a project. To add a product, the owner adds it to the config and runs setup.
5. `apply` adds each new initiative to the portfolio project that `plan.toml` names in `portfolio = "<key>"`. It adds only initiatives, never epics.

**How the tool tells them apart.** Setup writes a marker at the end of each project's README:

```
<!-- taskrock:project key=<key> kind=<delivery|portfolio> -->
```

Setup finds a project by the marker, so a rename does not create a second project (F4). `project_of` reads `readme` in its one GraphQL call and drops portfolio projects (F8). Exactly one project must be left. The messages are `the issue is in no delivery project: <url>` and `the issue is in more than one delivery project: <url>`. A project with no marker counts as a delivery project, so a project that setup has not touched keeps working. `ready` with a portfolio link stops with `<url> is a portfolio project. Give a delivery project or an initiative`.

## 3. The issue model

### Types

| Type | What it is | Built? |
|---|---|---|
| Initiative | a result worth weeks: why a body of work exists | no, it is a parent |
| Epic | one capability that can ship alone | no, it is a parent |
| Task | one step of planned work: one session, one branch, one pull request | yes |
| Bug | something that works wrong. It starts as a report. | yes, after triage |
| Feature | something that is missing. It starts as a request. | after triage: built like a Task, or it gets sub-issues and becomes a parent |

A Task with `Agent = Owner` is an owner task: work or a decision only the owner can do. It blocks the tasks that wait on it.

Planned issues (Initiative, Epic, Task) are written complete by the planner. Reported issues (Bug, Feature) are written incomplete by whoever hit the problem, so they have different starting sections and a triage step. Feature is kept as a type so a `type:Feature` view lists every request. GitHub creates the Feature type in every organization.

### Workflow

```
report ──► Triage ──► Todo ──► In Progress ──► Done
             │          ▲          │
             │          └─release──┘
             └──► closed "not planned", with the reason
```

| Move | Who | How |
|---|---|---|
| A Bug or Feature is filed | any agent or the owner, through a skill | `taskrock template <type>`, a one-file draft whose `parent` is a link under the goal and whose `plan.toml` has the goal's `milestone`, `apply`. Status Triage. |
| Triage → Todo | `taskrock-triage` | checks, reproduces, fills the build sections and routing, sets Todo |
| Triage → closed | `taskrock-triage` | duplicate, cannot reproduce, already built or rejected: closed as not planned with the reason |
| Needs information | `taskrock-triage` | stays in Triage with a comment "Established / Still needed" |
| A planned issue is created | `taskrock-plan` | straight to Todo |
| Todo → In Progress | `taskrock claim` | the only way work starts. No code is written for an issue that is not In Progress with its branch `feat/issue-<number>` on GitHub. The goal skill stacks a task whose blockers' pull requests are open with `taskrock claim --stacked`. |
| → Done | the "Pull request merged" workflow | the manager closes a parent when all its children are closed |

`ready` builds only Todo, so nothing in Triage is built by mistake. It still lists the Triage items apart, under `TRIAGE`, so a manager can see them. Every issue that an agent files while a goal is built (a deferred finding, a friction, a task for the owner) goes under the goal with the goal's milestone. The goal triages it, builds it, and is not done while it is open.

### Formats

Each type has two sets of sections. **File** sections are needed to create the issue and are checked by `check` and `apply`. **Build** sections are needed to take it and are checked by `ready` and `claim`. The sets live in the config, so the owner can change them. This table shows the defaults. A config with no `taskrock.toml` in `<owner>/.github` gets the same defaults.

| Type | File sections | Build sections |
|---|---|---|
| Initiative | Outcome, Why, Not yet specified, Out of scope, Done when | same |
| Epic | Outcome, Decisions already made, Not yet specified, Out of scope, Done when | same |
| Task | Outcome, Decisions already made, Steps, Done when, Out of scope | same |
| Bug | What happens, Expected, How to reproduce, Environment | the file sections, plus Cause, Decisions already made, Steps, Done when, Out of scope |
| Feature | Problem, Proposed behavior, Workaround today | the file sections, plus Decisions already made, Steps, Done when, Out of scope. A Feature with children is a parent and is not checked for build sections. |
| Owner task | Outcome, Question, Why it matters, Recommended answer, Done when | same |

Rules:

- A section that does not apply says `None.` and the reason. An empty section is a problem.
- **Done when** is a checklist. Each box is a command in backticks and the result it must show. `check` refuses a box with no command or no result. A command must be able to fail.
- A bare `#123` in a body is a problem. Name the key of the issue instead.
- A title starts with a capital letter, a digit or a code span in backticks. A lowercase letter is allowed only for the name of the repository, as in "taskrock 0.2.1: …".
- A Bug's **How to reproduce** is a command and its failing output. Its Done when includes that command passing and a regression test that failed before the fix.
- After a task is In Progress, Outcome, Decisions already made and Out of scope are frozen. Only `taskrock edit --replan` changes them.

## 4. The config file

The file is `taskrock.toml` in `<owner>/.github`, changed through pull requests. The commands read it with `gh api repos/<owner>/.github/contents/taskrock.toml`. `apply`, `ready`, `claim`, `edit` and `template --owner` use the built-in defaults when the file does not exist. `setup` needs the file. `--file <path>` reads a local copy instead. The parser rejects unknown keys.

```toml
version = 1

[routing]
runtimes   = ["claude", "codex", "agy", "muse", "grok"]
visibility = "restricted"          # who sees routing values: "restricted" or "all"

# Subagents: where each one is installed and what it can do.
# Agent offers "Owner" and the subagents that can build. Review agent offers those that can review.
[[routing.subagent]]
name     = "tailrocks-systems-engineer"
runtimes = ["claude", "codex", "muse"]
can      = ["build", "review"]

[[routing.subagent]]
name     = "tailrocks-code-reviewer"
runtimes = ["claude", "codex", "muse"]
can      = ["review"]

# Models: where each one runs, its efforts, and when the planner uses it.
[[routing.model]]
name     = "claude-haiku-5-5"
runtimes = ["claude"]
efforts  = ["low", "medium", "high", "xhigh", "max"]
use      = "Mechanical work: docs, renames, config, tests that follow an existing pattern."

[[routing.model]]
name     = "claude-sonnet-5-5"
runtimes = ["claude"]
efforts  = ["low", "medium", "high", "xhigh", "max"]
use      = "The default builder."

[review]
max_rounds = 3                     # reviewed heads before the task goes to the owner

# Issue types and their formats. Initiative, Epic, Task, Bug and Feature are required.
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

# ... Initiative, Epic and Feature in the same way.

# The owner task is not a GitHub issue type, so its format has its own table.
[owner_task]
file      = ["Outcome", "Question", "Why it matters", "Recommended answer", "Done when"]
build     = ["Outcome", "Question", "Why it matters", "Recommended answer", "Done when"]
checklist = ["Done when"]

[[project]]
key       = "taskrock"             # stable, written in the README marker
title     = "taskrock"
kind      = "delivery"
number    = 3                      # optional: adopt a project that exists
repos     = ["taskrock", "taskrock-plugins", "taskrock-sandbox"]

[project.workflows]                # the six defaults. false deletes one, for good (F11).
auto_add_sub_issues = true
item_added          = true
item_closed         = true
pr_linked           = true
pr_merged           = true
auto_close_issue    = true

[[project.milestone]]              # optional: one release in every repository of the project
title = "v0.2.0"
due   = "2026-12-15"

[[project]]
key   = "roadmap"
title = "Roadmap"
kind  = "portfolio"
```

The folder `crates/taskrock/fixtures/` holds a complete example, `config-good.toml`.

**What becomes a GitHub field.** Only the names. Subagent names fill Agent and Review agent, by `can`. Model names fill Model and Review model. All efforts of all models fill Effort and Review effort. Runtimes fill Runtime and Review runtime.

**What stays in the config.** Each block's `runtimes`, `efforts`, `can` and `use`. They are rules for the planner. taskrock does not check that a task's subagent, runtime, model and effort fit together, because a GitHub field cannot depend on another field. The planner picks a subagent, then a runtime from its list, then a model that lists that runtime, then an effort from that model.

**What the tool fixes.** It has no built-in subagent, runtime or model names. It fixes the routing field names, the Status options, the required issue types and the views of each kind, because the commands and skills depend on them.

**Checking the file.** `taskrock check <file>` checks the file offline and prints `OK config, <n> projects`, or one line for each problem. It checks that:

- `version` is 1 and `review.max_rounds` is at least 1.
- Subagent names, model names, issue type names and project keys are unique, and so are the milestone titles of a project.
- Each subagent and model runtime is in `routing.runtimes`. Each subagent has at least one `can` value, and each model has at least one effort.
- Each of the five required issue types is present, and the `file` and `checklist` sections of each format are also `build` sections.
- A project key has only letters, digits and `-`.
- A delivery project has at least one repository. A portfolio project has no repositories and no milestones.
- Each milestone due date is a real date `YYYY-MM-DD`.

The limit of 25 issue fields is checked by `setup` before it writes any field.

## 5. `taskrock setup`

```
taskrock setup <owner> [--file <path>] [--dry-run] [--yes]
```

Steps, in order:

1. **Read and check the file.** Print every problem and stop, before any write.
2. **Issue types.** Create each type in the file when missing.
3. **Routing fields.** Create each of the eight fields when missing. Add missing options with `PATCH orgs/<owner>/issue-fields/<id>`, sending the existing options with their IDs plus the new ones (F5). Never send options through GraphQL `updateIssueField`. Extra existing options are kept. If GitHub's default `Effort` field holds only the sizes `XS`, `S`, `M`, `L` and `XL`, rename it to `Size` first. If a field `Size` exists too, stop. If the organization would have more than 25 issue fields, stop before any field write. A new field gets the visibility `routing.visibility` says. An existing field whose visibility differs is changed with `update field <name> (visibility)`: `organization_members_only` for `restricted`, `all` for `all`.
4. **Projects.** Find each by marker, or by `number`. Create it when missing. Write the marker, set the title and keep it private, each with its own line: `update project <title> (marker)`, `(title)` or `(private)`. A project found by number whose marker has another key stops the run.
5. **Project fields.** Status gets Triage, Todo, In Progress and Done, sent with the existing IDs (F10). Triage is added after the existing options, which keep their order. The lines are `update status <project title> (add Triage)` and `exists status <project title>`. Each delivery project gets the eight routing fields as columns (F6), with the lines `create column <project title> <field>` and `exists column <project title> <field>`. Each portfolio project gets `Start date` and `Target date`.
6. **Views.** Turn "View 1" into the first missing view of the kind: name, filter, columns and, when it differs, layout. Create missing views. Set the layout, filter and columns of a view that differs, in one update, with a line such as `update view Ready (layout, filter, columns)`. Never delete a view, and list extra ones with `extra view <name>`. A column that is not a field of the project stops the run.
7. **Repositories.** Link each repository in `repos` (F7), with the lines `create link <project title> <owner>/<repo>` and `exists link ...`. List extra links with `extra link ...` and keep them.
8. **Milestones.** Create each shared milestone in each repository of its project, and fix its due date and description. The lines are `create milestone <repo> <title>`, `update milestone <repo> <title>` and `exists milestone <repo> <title>`. Milestones are matched by title, open or closed.
9. **Workflows.** Keep each workflow set to `true`. Delete each one set to `false`, only with `--yes`. If a `true` workflow is missing or off, stop with `project <title>: the workflow <name> is off and the API cannot turn it on`: the API cannot turn it on again. Check that each board groups by Status (F2), and note `view <name> groups by <names>, not Status. The API cannot change it` for one that does not.

Each change prints one line before it is made: `create`, `update`, `delete` or `exists`, then the kind and the name, for example `create field Agent`. A note about something setup cannot or will not change is a line of its own, for example `extra view Old`. The last line is `DONE <n> changes`, where `n` counts the lines that start with `create`, `update` or `delete`.

`--dry-run` writes nothing. In a dry run a project that does not exist yet is shown with `create project`, `update project <title> (marker)`, and `create` lines for its status, columns, views, links and milestones. It gets `exists` lines for the workflows set to `true`, because a new project has all six on (F3). In a dry run a view column that is not yet a field of an existing project is skipped, not an error. A second run on a set up organization prints only `exists` lines and notes.

### Views of each kind

| Kind | View | Layout | Filter | Columns |
|---|---|---|---|---|
| delivery | Board | board | `-type:Initiative,Epic` | Title, Agent, Model |
| delivery | Ready | table | `is:open -is:blocked status:Todo -agent:Owner no:sub-issues-progress` | Title, Status, Milestone, Agent, Runtime, Model, Effort |
| delivery | Triage | table | `status:Triage` | Title, Status, Repository, Milestone |
| delivery | Needs you | table | `is:open agent:Owner` | Title, Status, Milestone, Parent issue |
| delivery | Requests | table | `type:Feature` | Title, Status, Repository, Milestone |
| delivery | All tasks | table | `-type:Initiative,Epic` | Title, Status, Milestone, Agent, Runtime, Model, Effort, Review agent, Review runtime, Review model, Review effort |
| delivery | Tree | table | none (flat: nesting cannot be set, F9) | Title, Status, Milestone, Sub-issues progress, Agent |
| portfolio | Initiatives | table | `type:Initiative` | Title, Status, Sub-issues progress, Start date, Target date |
| portfolio | Roadmap | roadmap | `type:Initiative` (default dates, F9) | Title, Status, Sub-issues progress |

A view that is there with another layout, filter or columns gets one update that fixes all of them: setup prints `update view <name> (<parts>)`, where `<parts>` lists each of `layout`, `filter` and `columns` that differs, in that order, for example `(filter)` or `(layout, filter)`. "View 1" is renamed to the first missing view and gets its layout in the same update. A view with extra visible columns keeps them.

### Workflows

The config keys and the names that GitHub shows:

| Key | Workflow |
|---|---|
| `auto_add_sub_issues` | Auto-add sub-issues to project |
| `item_added` | Item added to project |
| `item_closed` | Item closed |
| `pr_linked` | Pull request linked to issue |
| `pr_merged` | Pull request merged |
| `auto_close_issue` | Auto-close issue |

A workflow set to `false` is deleted for good with `--yes`, with the line `delete workflow <name>`. Without `--yes`, setup prints `workflow <name> is set to false. Run again with --yes to delete it for good` and goes on. A workflow that is already gone prints `exists workflow <name> (deleted)`.

## 6. The commands

| Command | What it does |
|---|---|
| `setup` | New. Section 5. |
| `template <type>` | Prints an empty body with the file sections of the type, read from the config. `--owner <owner>` reads `<owner>/.github/taskrock.toml`, `--config <file>` reads a local file, and with neither it uses the defaults. `template owner` prints the owner task. A checklist section gets an empty box. |
| `edit <link> <file>` | Checks the body against the type, then writes it. A Triage issue or one with no status is checked against the file sections, any other against the build sections, and an owner task against the owner-task format. After In Progress, the sections Outcome, Decisions already made and Out of scope are frozen: a change needs `--replan`, which also posts a comment that names the changed sections. The hidden `<!-- taskrock:` marker lines are kept. The file has no front matter. |
| `check <path>` | Checks a draft folder, or a config file (section 4). `--config <file>` reads the issue formats from a config file, offline, instead of the defaults. It refuses a Done-when box with no command or no result, a bare `#123` in a draft, a `parent` or `blocked_by` that is neither a key of the draft nor a full issue link `https://github.com/<owner>/<repo>/issues/<number>`, and a routing field on a parent. It also refuses a title that starts with a lowercase letter, unless the title starts with the name of the repository of the issue followed by a space or a colon. A Task has all eight routing fields or `Agent` `Owner` alone. A Bug or Feature has all eight or none. |
| `check <folder> --coverage` | A read-only report. It notes a parent whose Outcome has more list items than the parent has children, a task with more than 10 Steps, a task or bug with no builder or no reviewer, and a reviewer that runs on the builder's model. |
| `apply` | Checks the file sections from the config. Puts each Bug and each Feature with no children in Triage, and each other new issue with no status in Todo, with the line `status: #<number> <status>`. It reads the statuses again after a pause and sets Triage once more where the "Item added to project" workflow set Todo. Adds initiatives to the portfolio project named in `plan.toml`, with the line `portfolio: added #<number>`. A project with no Status option Triage stops it when the draft has a Bug, or a Feature with no children. A `portfolio` key that matches no portfolio project stops it as well. Both messages say to run `taskrock setup`. `--dry-run` prints `would set <key> to Triage` and `would add <key> to portfolio <portfolio key>`. A `parent` or `blocked_by` that is a full issue link points at an issue that exists: `apply` reads it with `GET` for its `id`, creates the new issue as its sub-issue or blocks it by that issue, and stops with `no issue <link>` when it does not exist and with `<link> is a pull request, not an issue` for a pull request. `--dry-run` prints `would set parent of <key> to <link>` and `would block <key> by <link>`. An issue whose `parent` is a link is added to the project at once, like an issue with no parent. The optional `milestone = "<title>"` of `plan.toml` is set on every created issue that has no `milestone` of its own. `apply` finds it by title with `GET repos/<owner>/<repo>/milestones?state=all` in the repository of each issue, and stops before it creates anything with `no milestone <title> in <owner>/<repo>. Run taskrock setup` when one is missing, in every repository, before the first milestone or issue is created. A milestone that an issue of the repository names and `apply` creates counts as present, in a dry run too. `--dry-run` prints `would set milestone <title> on <key>`. |
| `ready` | Lists a task whose body breaks the build sections of its type under problems, and leaves it out of the ready list. Warns when two ready tasks name the same file in Decisions already made. Accepts Feature. Refuses a portfolio link. Lists the open issues under the link whose Status is Triage in a `TRIAGE <n>` section and in the `--json` key `triage`, each `{number, title, url, type}`. A Triage item is not a candidate and not a problem, so a manager sees what was reported and none of it is built. With an issue link whose issue has a milestone, it lists the open Task, Bug, Feature and Epic items of the same project that have the same milestone and no parent, and are not the link itself, in a `LOOSE <n>` section (printed only when `n` is above 0, after `TRIAGE`) and in the `--json` key `loose`, each `{number, title, url, type}`. A loose item is not a candidate and not a problem, so a manager sees an issue that was filed outside its tree. A project link or a link with no milestone has no loose items. With `--stacked` it also lists a Todo task whose open blockers form one stack, and every ready entry of `--json` has the key `stacked: null`, or `{base, blocker, pull_request}`. |
| `claim` | Refuses a task whose body breaks its build sections. Refuses an issue in Triage with "the issue is in Triage. Triage it first with the skill taskrock-triage", and an issue with open blockers with "<n> open issues block it. Build it only after they close". `--stacked` takes a task whose open blockers form one stack: it starts the branch from `feat/issue-<blocker>`, posts a comment with the marker `<!-- taskrock:stacked-on feat/issue-<blocker> -->`, and prints a third line `base <branch>`. It refuses a blocker in another repository, a blocker that is not In Progress, a blocker with no open pull request, and blockers that are not one stack. |
| `release --note <file>` | Posts a handoff comment first, then gives the task back. The note has the sections Done, Verified, State and Next step. The comment starts with `_Written by an agent._`. |
| `project_of` | An internal call that drops portfolio projects. |

`apply`, `ready`, `claim` and `edit` read the config from `<owner>/.github`. The command still holds only what `gh` cannot do: `gh` cannot set issue fields, check a body against a format, or tell project kinds apart.

## 7. The skills

All skills read the config. No skill calls `gh issue create` or `gh issue edit --body`. `scripts/check.py` in taskrock-plugins fails when a skill names either. The version check asks for 0.4.0. A skill builds an issue only after taskrock claim took it, and never builds an issue in Triage.

The goal skill lists ready tasks with `taskrock ready --stacked`, takes a stacked task with `taskrock claim --stacked`, merges a stack from the bottom up, and retargets each pull request to the default branch before its base merges.

| Skill | What it does |
|---|---|
| `taskrock-plan` | Asks its questions in rounds, each multiple choice with a recommended answer. Finds facts by running commands and never asks them. Reviews its own tree before it shows it. Writes Done-when boxes as commands that can fail. Splits a wide change into prefactor, expand, migrate and contract. Gives each parent Not yet specified and Decisions so far. Takes routing only from the config blocks, with `use` guiding the model choice. Picks the delivery project from the config. It counts filed Bugs in the Done when of an initiative, and ends by naming the skill that builds the plan. |
| `taskrock-issue` | Reads the issue's comments first. Merges the default branch and reruns Done when before the pull request. Ends with DONE, BLOCKED or TASK_WRONG. Uses `taskrock release --note` when it stops. It files a friction under the parent of its task, with the task's milestone. |
| `taskrock-goal` | The verdict names the commit: `VERDICT: PASS head=<sha>`. Only findings that the reviewer reproduced count, and each is routed as a fix, a decision (an owner task) or a deferral (a Bug in Triage). It allows at most `review.max_rounds` reviewed heads before the task goes to the owner. No reviewer available means no merge. When an epic closes, it makes an owner task with the repeated failures and proposed `AGENTS.md` edits. It appends Decisions so far to parents. It files every issue under the goal with the goal's milestone, sweeps the Triage items of the goal before it stops, and says that the work is done only when the goal's milestone has no open issue. |
| `taskrock-triage` | Checks for an existing implementation, reproduces a bug and searches closed requests. A report that needs information stays in Triage with a comment and no label. A rejected request is closed as not planned with the reason. Agent comments say an agent wrote them. |
| `taskrock-setup` | Interviews the owner in rounds with recommended answers, reading the facts of the organization with `gh api`. Offers only runtimes and models found on the machine. Writes the config, opens the pull request to `<owner>/.github`, runs `taskrock setup --dry-run`, then `taskrock setup` after approval. |

## 8. Facts proven on 2026-11-26

Each fact was tested in tailrocks with projects, an issue field and an issue made for the test, all deleted afterwards. [Facts](facts.md) has the rows.

| # | Fact | Result | What setup does |
|---|---|---|---|
| F1 | Views through the API | **Works.** `createProjectV2View` sets name, layout and `visibleFieldIds`. `updateProjectV2View` sets the filter, the columns and the layout. There is no REST endpoint for views. | Creates and fixes views with GraphQL. |
| F2 | Board grouping | **Works by default.** A board made through the API groups its columns by Status. The API cannot set any group-by or sort. | Checks it. |
| F3 | Workflows of a new project | **All six are on.** A new project also has one table view "View 1", is private, and has Status Todo, In Progress and Done. | Renames "View 1". Checks the workflows. |
| F4 | README marker | **Works.** An HTML comment in the README comes back through the API exactly as written. | Finds projects by the marker. |
| F5 | Adding an issue-field option | **GraphQL `updateIssueField` replaces the whole list.** Sending only the new option deleted the old ones and wiped the value on the issue. Sending all names fails with "Name has already been taken". **REST `PATCH /orgs/<org>/issue-fields/<id>` with the existing options' `id`s plus the new option keeps every option and every value.** | Adds options only with the REST call and the existing IDs. |
| F6 | Issue field in a project | **Works.** `createProjectV2IssueField` adds an org issue field to a project as a column. | Adds the routing fields to delivery projects. |
| F7 | Linking a repo twice | **Works.** The second `linkProjectV2ToRepository` succeeds with no error. | Reads the linked repositories and links only the missing ones. |
| F8 | An issue in two projects | **Works.** Each project's REST items call lists the issue with its own Status. One GraphQL call returns both projects with their `readme`. | Portfolio projects are allowed. |
| F9 | Roadmap dates, Tree nesting | **Not settable.** The view configuration has only `visibleFields`. | Roadmap uses default dates. Tree is flat. |
| F10 | Adding a Status option | **Works.** `updateProjectV2Field` with the existing options' `id`s plus "Triage" kept the item's Status. | Adds Triage with the IDs. |
| F11 | Deleting a workflow | **Works**, and it cannot be turned on again through the API. | Deletes only with `--yes`. |

## 9. Left out on purpose

- Deleting types, fields, options, views, links, milestones or projects.
- Copying a project, and any step in the browser.
- Labels, rulesets, branch protection and repository settings.
- Personal-account projects.
- Moving issues between projects, and moving the old trackers to this setup.
- A scheduled setup run in CI.
