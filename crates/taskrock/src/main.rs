//! The taskrock command line.

use clap::Parser as _;
use taskrock::cli::Cli;

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    match taskrock::run(&cli) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            std::process::ExitCode::from(error.exit_code())
        }
    }
}
