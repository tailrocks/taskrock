//! The command line of taskrock.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

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
        /// The draft folder, or a `taskrock.toml` file.
        draft: PathBuf,
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
    },
    /// Take a task.
    Claim {
        /// The link of the task.
        link: String,
    },
    /// Give a task back.
    Release {
        /// The link of the task.
        link: String,
    },
}
