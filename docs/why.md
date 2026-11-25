# Why taskrock exists

The owner plans work for AI coding agents on GitHub. That work lived in three trackers. Each recorded the same facts in a different way, so the trackers drifted from the truth. taskrock is one way to plan and run that work, the same in every organization. GitHub is the only store, and a small Rust command covers the steps that `gh` cannot do safely.

## The problem

The owner directs several agents in several agent CLIs. Their work was planned in three GitHub projects: one under a personal account and two under organizations. The table shows the state on 2026-11-22.

| | Tracker A | Tracker B | Tracker C |
|---|---|---|---|
| Items (open) | 378 (141) | 333 (109) | 326 (6), in 16 repositories |
| Model recorded as | A label and a project field | An `exec:` label and body text | A label, a project field and body text |
| "In progress" recorded as | The project Status, set by hand | An `in-progress` label | Derived from open pull requests |
| Tool that wrote the issues | A generator | None | A sync script |

The same fact had a different home in each tracker. The drift that followed:

- 101 closed issues in tracker B still carry the `in-progress` label.
- The root issue of tracker B has 94 direct sub-issues. GitHub allows 100 per parent.
- One issue in tracker A is a duplicate that two sync runs created at the same moment.
- In one repository of tracker C, which the sync script drives, 56 of 65 issues had an edited body. 42 of them had it edited five times or more. Of the 100 newest issues, 5 in tracker A and 2 in tracker B had an edited body.

Every sampled edit and comment in the three trackers came from one login. GitHub cannot tell two agents apart in that setup. So an assignee does not say who took a task.

## What was missing

GitHub has the parts of a good plan: issues, sub-issues, blockers, milestones, projects, issue types and issue fields. What it lacks sits between those parts. The first bullet was measured on 2026-11-22 with `gh` 2.105.0. The others were found while building taskrock.

- `gh issue create` and `gh issue edit` set the type, the parent and the blockers. They have no flag for issue fields. The REST API sets the fields with their IDs and the single-select values by option name.
- Nothing checks a plan before it is written. GitHub rejects a blocker cycle of two issues and accepts a cycle of three.
- Creating a plan needs an order, a map from the keys of the draft to the numbers of the new issues, and a rerun that creates nothing twice. A listing shows a new issue only 6 to 8 seconds after it is created, and the search lags behind writes.
- Finding the tasks that can start now needs the children of children. `gh` cannot select them.
- Two agents must not take the same task. Only the REST call that creates a branch refuses the second creator. `gh issue develop` and the GraphQL call do not.
- A claim has several writes. A claim that stops halfway must be undone, and a task that is given back changes what its parents show.
- GitHub does not close a parent when its last child closes.
- Three agents used up the hourly GraphQL budget of one account in one hour. A tool that agents call all day must use REST.

The skill catalogs that were read do not fill these gaps. They were: `tailrocks/tailrocks-skills` (10 packages, 98 skills) and two other catalogs (43 and 39 skills). None of them creates projects, milestones, issue types, issue fields or routing on GitHub. No existing Rust tool was fit to adopt either. The [comparison](comparison.md) names the projects that were studied.

## What taskrock is

GitHub is the only place work lives. `taskrock` holds only what is hard, almost impossible or not convenient with `gh`. Everything else is plain `gh`, written in a skill.

It has three parts:

- A Rust command with five subcommands. It checks a plan, creates it on GitHub, lists the ready tasks, takes a task and gives it back.
- Three skills. They tell an agent how to plan, how to work on one task and how to manage a goal.
- The setup of a GitHub account: the issue types and the eight routing fields that say which agent builds a task and which agent reviews it.

The planner uses the strongest model. It writes each task so that a weaker model makes no design decision. Builders and reviewers then run on the model and in the agent CLI that each task names. The [design](design.md) page gives the principles and the reasons for each choice. The [structure](structure.md) page shows where each part lives. The [README](../README.md) has the install steps and the commands.

## Who it is for

One person who directs several agents. The agents run in several agent CLIs and work in more than one GitHub organization. The owner uses taskrock in three ways:

- **Plan.** Explain the work to the strongest agent and run the plan skill. Approve the breakdown. The agent creates everything on GitHub and prints the links.
- **Build.** Start a new session in any CLI and type `/goal Implement <link>`. The session works until everything under the link is done or only owner tasks are left.
- **One task.** Type `Do <link>` in a session.

## What it is not

- Not a tracker of its own. Every issue lives on GitHub and nowhere else. See [design](design.md).
- Not a mirror of GitHub in files. A draft is applied once and is not read again. See [design](design.md).
- Not a tool that starts agents. A skill lists the launch command of each CLI. See [design](design.md).
