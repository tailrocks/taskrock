# Rules

The plan for this repository is on GitHub: the issues of `tailrocks/taskrock` and the project `https://github.com/orgs/tailrocks/projects/3`. One issue is one task. Build only what the task says.

## The task is the contract

- The names in a task are binding: files, modules, types, functions, commands, messages. Do not rename them.
- Do not make a design decision. If the task is wrong or something is missing, stop, write what you found as a comment on the issue, and give the task back.
- Stay inside the task. Its `Out of scope` section is a hard limit.
- A box under `Done when` is done only after you ran its command and saw the result.

## Code

- One crate: `crates/taskrock`. The library holds the logic. `src/main.rs` only parses arguments and calls the library.
- No `unwrap`, `expect`, `panic`, `todo` or `unimplemented` outside tests. Return `Result`.
- A function is at most 80 lines. A Rust file is at most 400 lines. `lib.rs`, `main.rs` and every `tests.rs` are at most 150 lines.
- Module files are named after the module (`link.rs`, then `link/` for its children). No `mod.rs`.
- Unit tests live in `src/<module>/tests.rs`, declared in the module with `#[cfg(test)] mod tests;`. Integration tests live in `crates/taskrock/tests/`, which holds only `.rs` files. JSON test data lives in `crates/taskrock/fixtures/`.
- Every GitHub request goes through the `Gh` trait. Use the REST API. Do not use the search API: its results lag behind writes.
- GraphQL is allowed only where REST has no equivalent: the resolved state of review threads, `resolveReviewThread`, the merge queue (`mergeQueue`, `mergeQueueEntry`, `enqueuePullRequest`), and Projects v2 fields, views and items (`createIssueField`). The code also makes these Projects v2 calls: the `projectsV2` and `projectV2` queries, `createProjectV2`, `updateProjectV2`, `updateProjectV2Field`, `createProjectV2IssueField`, `createProjectV2View`, `updateProjectV2View`, `linkProjectV2ToRepository`, `deleteProjectV2Workflow`, and the `projectItems` field of an issue. Use `github::graphql` for it.
- Add a dependency only when the task names it. Add it with `cargo add`, and commit `Cargo.lock`.
- A test that needs a folder uses `taskrock::scratch::Scratch`, which removes the folder on drop. Never call `std::env::temp_dir()` in a test.

## Checks

Run `mise run check` before every commit. It runs the format check, clippy with warnings as errors, the tests, the doctests, the dependency checks, the repository policy and the generated-workflow check.

Never edit a file under `.github/workflows/`. They are generated. Change `.velnor/rust-ci.toml` and run `velnor-gen --root . generate`.

## Git and pull requests

- Work on the branch of the task: `feat/issue-<number>`. It already exists on GitHub when you start. Fetch it and switch to it. Do not create another branch.
- Commit subjects follow Conventional Commits. Sign off every commit: `git commit -s`.
- Push often.
- One task, one pull request. Its body has the sections `Why`, `What`, `Proof` and `Done when`, and the line `Closes #<number>`. `Proof` holds each `Done when` command with the output you saw.
- Do not merge. The manager merges after the review passes.
