//! The command line of taskrock.

use std::path::PathBuf;

use clap::{ArgGroup, Parser, Subcommand};

/// Plan and run agent work on GitHub Issues and Projects.
#[derive(Debug, Parser)]
#[command(name = "taskrock", version, arg_required_else_help = true)]
pub struct Cli {
    /// The command to run.
    #[command(subcommand)]
    pub command: Command,
}

/// The commands of taskrock.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Check a draft or a config file against the rules. Uses no network.
    Check {
        /// A draft folder, or a taskrock.toml file.
        path: PathBuf,
        /// Print a read-only coverage report of the draft instead.
        #[arg(long)]
        coverage: bool,
        /// Read the issue formats from this config file instead of the built-in ones.
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Create the plan of a draft on GitHub.
    Apply {
        /// The draft folder.
        draft: PathBuf,
        /// Print what would be created and create nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// List the tasks that can start now.
    Ready {
        /// The link of an initiative, an epic, a task or a project.
        link: String,
        /// Print JSON.
        #[arg(long)]
        json: bool,
        /// Also list a task whose open blockers are In Progress with open pull
        /// requests, and name the branch to stack on.
        #[arg(long)]
        stacked: bool,
    },
    /// Take a task.
    Claim {
        /// The link of the task.
        link: String,
        /// Start from the branch of the open blocker whose pull request is open,
        /// instead of the default branch.
        #[arg(long)]
        stacked: bool,
    },
    /// Give a task back.
    Release {
        /// The link of the task.
        link: String,
        /// Post this file as a handoff comment first.
        #[arg(long)]
        note: Option<PathBuf>,
    },
    /// Make GitHub match the organization's taskrock.toml.
    Setup {
        /// The organization.
        owner: String,
        /// Read this file instead of `<owner>/.github/taskrock.toml`.
        #[arg(long)]
        file: Option<PathBuf>,
        /// Print what would change and change nothing.
        #[arg(long)]
        dry_run: bool,
        /// Also delete the workflows set to false.
        #[arg(long)]
        yes: bool,
    },
    /// Print an empty body with the file sections of a type.
    Template {
        /// The issue type, or `owner` for an owner task.
        #[arg(value_name = "TYPE")]
        kind: String,
        /// Read the formats from `<owner>/.github/taskrock.toml`.
        #[arg(long)]
        owner: Option<String>,
        /// Read the formats from this file.
        #[arg(long, conflicts_with = "owner")]
        config: Option<PathBuf>,
    },
    /// Replace the body of an issue after checking it against its type.
    Edit {
        /// The link of the issue.
        link: String,
        /// A Markdown file with the new body, without front matter.
        file: PathBuf,
        /// Allow changes to the frozen sections of a task in progress.
        #[arg(long)]
        replan: bool,
    },
    /// Work with pull requests.
    Pr {
        #[command(subcommand)]
        command: PrCommand,
    },
}

/// The commands of `taskrock pr`.
#[derive(Debug, Subcommand)]
pub enum PrCommand {
    /// Print the state of a pull request.
    Status {
        /// The link of the pull request.
        link: String,
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
    /// List the review threads of a pull request, and defer or resolve them.
    #[command(group(ArgGroup::new("action").required(true).args(["defer", "resolve"])))]
    Threads {
        /// The link of the pull request.
        link: String,
        /// Print JSON.
        #[arg(long)]
        json: bool,
        /// Defer the open threads.
        #[arg(long, conflicts_with = "resolve")]
        defer: bool,
        /// Resolve the thread with this id.
        #[arg(long, requires = "reason")]
        resolve: Option<String>,
        /// Why the thread is resolved.
        #[arg(long, requires = "resolve")]
        reason: Option<String>,
    },
    /// Merge a pull request.
    Merge {
        /// The link of the pull request.
        link: String,
        /// Print JSON.
        #[arg(long)]
        json: bool,
    },
}
