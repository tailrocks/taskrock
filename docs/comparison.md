# Compared with the projects that were studied

No project that was read creates the structure of a plan on GitHub: the project, the milestones, the issue types, the fields and the routing. None of them hands a task to a different agent CLI. taskrock does both: `apply` creates the structure, and the goal skill starts each builder in the CLI that its task names. It leaves the rest to `gh` and to GitHub's own features. The three skills of taskrock are self-contained and call no skill of another package.

The skill catalogs were read on 2026-11-22. The public pages below were opened again on 2026-11-25. Counts of skills are left out because they change.

## At a glance

| Project | What it is | Where the plan lives | What taskrock took | Where taskrock differs |
|---|---|---|---|---|
| `gh` and GitHub features | The official command line tool, and the features it drives | On GitHub | All of it. Every request goes through `gh` | Adds five commands for what `gh` cannot do safely |
| `mitsuhiko/gh-issue-sync` | A mirror of issues as Markdown files, for one repository | In local files and on GitHub, kept in step | Front matter at the top of a draft file, and temporary keys | The draft is used once. GitHub holds the only copy |
| `mattpocock/skills` | A catalog of skills for engineering work | In the issue tracker the user picks, or in local files | The idea of tickets that declare what blocks them | GitHub only. taskrock sets up the structure and routes each task. No skill of it is called |
| pstack (`michael-denyer/pstack-claude`) | A skill plugin with workflows for agent harnesses | Its page names no plan store | Studied. No pstack skill is called in 0.1.0 | taskrock plans the work around the tasks and does not describe how an agent builds one |
| `tailrocks/tailrocks-skills` | The marketplace of Tailrocks skill packages | In files, in the roadmap package | Studied. No skill of another package is called in 0.1.0 | The skills of taskrock are one package of it. Plans live on GitHub |
| An earlier sync script | A script that pushed one plan file to GitHub | In one plan file | Prior art for pushing a plan | The plan file is not the master. It is applied once |

## gh and GitHub's own features

`gh` is GitHub's official command line tool (`cli/cli`, MIT, checked on 2026-11-25). GitHub itself has sub-issues, issue types, issue fields, blockers, milestones and projects.

taskrock is built on them. It stores no fact of its own. The tool calls `gh` for every request, so there is one login. Most jobs stay plain `gh` commands inside a skill, such as `gh issue edit --add-blocked-by`. Version 0.2.0 adds `taskrock setup`, which sets up the projects of an organization through the API.

taskrock goes another way for five jobs. With `gh` 2.105.0 on 2026-11-25, `gh issue create` and `gh issue edit` set the type, the parent and the blockers, and have no flag for issue fields. Nothing in `gh` checks a plan before it is written, and GitHub accepts a blocker cycle of three issues. `gh` cannot select the children of children, and its search lags behind writes. Two agents that take the same task need a branch name that only one of them can create. These jobs are the commands `check`, `apply`, `ready`, `claim` and `release`. [Why the project exists](why.md) has the full list of gaps.

## mitsuhiko/gh-issue-sync

`gh-issue-sync` is a command line tool in Go under Apache-2.0 (checked on 2026-11-25). It mirrors the issues of one repository into a `.issues/` folder as Markdown files with YAML front matter. A person or an agent edits the files, and `push` and `pull` carry the changes both ways. It compares the local file, the saved original and the remote issue, and it skips a conflict with a warning. New issues get temporary IDs such as `T1`, and the tool replaces them with real numbers after the push. Its front matter lists, among other things, labels, milestone, type, projects, parent and blockers.

taskrock took two ideas. A draft issue is a Markdown file with front matter at the top. A draft issue has a temporary key that stands for the real number until the issue exists, and `apply` keeps a map from keys to numbers. In taskrock the front matter is TOML between `+++` lines, because the Rust repositories of Tailrocks already use the `toml` crate and this avoids a YAML dependency. A key names a parent or a blocker, so the draft is complete before any issue exists.

taskrock does not keep the mirror. The draft folder is temporary. The plan skill deletes it after `apply`. After that, an edit on GitHub is an edit to the only copy, so no conflict can arise. The front matter of `gh-issue-sync` does not list issue fields, so it has no place for the eight routing fields. The tool has no command to claim a task or to list the tasks that are ready. It is also written in Go, and no existing Rust tool was fit to adopt.

## mattpocock/skills

`mattpocock/skills` is a catalog of agent skills for engineering work (MIT, checked on 2026-11-25). Its page says the skills are small and composable. It has a grilling skill that questions the user about a plan, `to-spec` that turns a conversation into a spec, `to-tickets` that breaks a plan into tickets with blocking edges, `wayfinder` for decision tickets, and `implement-spec` that runs implementer subagents across the ready tickets. Its setup skill asks which issue tracker to use: GitHub, GitLab, local files or another.

The method of `to-spec` and of `to-tickets` was studied during the design. One idea of `to-tickets` is visible in `taskrock-plan`. Its step 3 splits the work into an initiative, epics and tasks, and gives a blocker to two tasks that change the same file. Each task names its blockers in `blocked_by` in the draft. The planner of taskrock does not ask a new round of questions after the conversation. Its step 1 asks only when a wrong guess would change the breakdown and nothing settles it, not the user's words, not the code, not the docs. It asks all open questions once, each with a suggested answer. It has no separate grilling skill. The task format of taskrock is its own, with five sections from Outcome to Out of scope.

taskrock goes another way in three places. It works with GitHub only, so the skills hold the real `gh` commands. It sets up the structure itself: issue types, eight routing fields and a project with views. And every task names its builder and its reviewer by agent, runtime, model and effort. Each task has its own branch and its own pull request, where `implement-spec` works a whole spec on one integration branch.

## pstack

pstack is a skill plugin. The repository `michael-denyer/pstack-claude` ports an upstream Cursor skill stack to Claude Code, Codex, Pi, GitHub Copilot, Muse and other harnesses (MIT, checked on 2026-11-25). Its page lists playbooks for planning, features, bug fixes, refactoring, shipping and longer projects. It has project playbooks that customize a workflow, and a setup skill that picks a model and a reasoning effort per role.

pstack was studied during the design, and the use of its playbooks for the build was considered. Version 0.1.0 does not use them. The skills of taskrock do not mention pstack and call none of its skills. `taskrock-issue` holds the whole path of one task in one skill: read the issue, take it with `taskrock claim`, build by the Steps, prove by the Done when list and open the pull request. taskrock does not fork pstack and does not copy its playbooks.

The two do different jobs. pstack describes how an agent does a piece of work and how it checks the result. taskrock handles the plan around the tasks: which task is ready, who builds it, who reviews it and who merges it. The page of pstack names no store for plans.

## tailrocks/tailrocks-skills

`tailrocks/tailrocks-skills` is the marketplace of Tailrocks skill packages (Apache-2.0, checked on 2026-11-25). Each package is its own repository, and each catalog entry names the repository and an exact commit.

The skills of taskrock were one package of it, `tailrocks/tailrocks-taskrock-skills`, until they moved to their own marketplace, `tailrocks/taskrock-plugins`. The three skills call no skill of another package. Three ideas from other packages were considered and are not in version 0.1.0: a grilling skill for the question round, the task format of the `tailrocks-plan` skill, and the pull request skill `tailrocks-pr-create`. In their place, `taskrock-plan` has its own question rule and its own task format. `taskrock-issue` writes `Closes #<number>` into the body of the pull request itself and opens it with `gh pr create`.

The roadmap package carries an idea through shaping, planning and proof, and keeps its plans as files. taskrock leaves that package untouched. The plan of taskrock lives on GitHub, and each task carries routing fields.

## An earlier sync script

One of the three existing trackers was fed by a private script. It pushed one plan file to GitHub in eight stages, with calls to `gh api`. The script is not public and is not linked here.

It served as prior art for putting a plan on GitHub with `gh`. taskrock does not take its design. In the script, the plan file is the master and the script writes one way. In a sample, the issues of the tracker it fed were edited after creation far more often than in the other two trackers. taskrock applies a draft once, so the issue on GitHub is the only copy. A rerun of `apply` creates nothing twice.

## Three designs that were considered for taskrock itself

- A permanent two-way mirror in a board repository. Files and GitHub would both hold each issue and a sync tool would reconcile them.
- One-way plan files as the master. A file would hold the plan, and each change would be pushed to GitHub.
- An MCP server that holds state in memory. Agents would talk to the server instead of GitHub.

taskrock uses none of them. GitHub is the only store, a Rust command checks and applies a plan, and skills tell the agent which command to use. The [design](design.md) page holds the reasons.
