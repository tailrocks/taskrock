# taskrock

GitHub is the only place work lives: issues, projects and pull requests. `taskrock` holds only what is hard, almost impossible or not convenient with the `gh` command.

## Install

```toml
# mise.toml
[tools]
"cargo:https://github.com/tailrocks/taskrock" = { version = "tag:v0.1.0", crate = "taskrock" }
```

The tool needs Rust 1.100.0 or newer.

The skills are the native plugin `tailrocks-taskrock-skills` of the Tailrocks marketplace. Its [installation guide](https://github.com/tailrocks/tailrocks-taskrock-skills/blob/main/docs/installation.md) has the command for each agent CLI. For Claude Code:

```sh
claude plugin marketplace add tailrocks/tailrocks-skills
claude plugin install tailrocks-taskrock-skills@tailrocks --scope user
```

Install the skills only as a plugin. Do not copy skill folders by hand. Remove old copies named `taskrock-*` from `~/.agents/skills` and `~/.claude/skills`.

## Set up an account

Run it once for each organization. It creates only what is missing.

```sh
#!/usr/bin/env bash
# Creates the issue types and the eight routing fields of taskrock in one
# organization. It creates only what is missing, so it is safe to run again.
set -euo pipefail
owner="$1"
owner_id=$(gh api graphql -f query='query($o:String!){organization(login:$o){id}}' -f o="$owner" --jq .data.organization.id)

have_types=$(gh api "orgs/$owner/issue-types" --jq '.[].name')
for type in Initiative Epic; do
  if grep -qx "$type" <<<"$have_types"; then echo "exists  type $type"; continue; fi
  gh api graphql -f query='mutation($o:ID!,$n:String!){createIssueType(input:{ownerId:$o,name:$n,isEnabled:true}){issueType{name}}}' -f o="$owner_id" -f n="$type" >/dev/null
  echo "created type $type"
done

roles="tailrocks-systems-engineer tailrocks-mechanic tailrocks-docs-writer tailrocks-code-reviewer"
runtimes="claude codex agy muse opencode mimo minimax kimi grok"
models="muse-spark-1.3 muse-ember-1.0 muse-wonder-1.0 muse-wonder-full-1.0"
efforts="minimal low medium high xhigh max"

options() { # prints a GraphQL list of options from words
  local i=0 out="" word
  for word in "$@"; do i=$((i + 1)); out+="{name:\"$word\",color:GRAY,priority:$i},"; done
  echo "[${out%,}]"
}

have_fields=$(gh api "orgs/$owner/issue-fields" --jq '.[].name')
field() { # field <name> <description> <option words...>
  local name="$1" description="$2"; shift 2
  if grep -qx "$name" <<<"$have_fields"; then echo "exists  field $name"; return; fi
  gh api graphql -f query="mutation(\$o:ID!,\$n:String!,\$d:String!){createIssueField(input:{ownerId:\$o,name:\$n,description:\$d,dataType:SINGLE_SELECT,options:$(options "$@")}){issueField{... on IssueFieldSingleSelect{name}}}}" -f o="$owner_id" -f n="$name" -f d="$description" >/dev/null
  echo "created field $name"
}
field "Agent" "Who builds the task: a role, or Owner for work only the owner can do." Owner $roles
field "Runtime" "The agent CLI that builds the task." $runtimes
field "Model" "The model that builds the task." $models
field "Effort" "The reasoning effort of the builder." $efforts
field "Review agent" "Who reviews the pull request before it merges." $roles
field "Review runtime" "The agent CLI that reviews the pull request." $runtimes
field "Review model" "The model that reviews the pull request." $models
field "Review effort" "The reasoning effort of the reviewer." $efforts
```

A new project is a copy of a project that already has the fields and the views:

`gh project copy <number> --source-owner <owner> --target-owner <owner> --title "<title>"`

## Use

The owner works in three ways. To plan, explain the work, then run the skill `taskrock-plan`. In Claude Code that is `/tailrocks-taskrock-skills:taskrock-plan`. To build, type `/goal Implement <link>`. For one task, type `Do <link>`.

## Commands

| Command | Description |
| --- | --- |
| `check` | Check a draft against the rules. Uses no network |
| `apply` | Create the plan of a draft on GitHub |
| `ready` | List the tasks that can start now |
| `claim` | Take a task |
| `release` | Give a task back |

## Develop

```sh
mise install
mise run check
```

## License

Apache-2.0. See [LICENSE](LICENSE).
