//! The taskrock command line.

use clap::Parser as _;
use taskrock::cli::Cli;

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    match taskrock::run(&cli, &taskrock::gh::RealGh::default()) {
        Ok(code) => std::process::ExitCode::from(code),
        Err(error) => {
            eprintln!("error: {error}");
            std::process::ExitCode::from(error.exit_code())
        }
    }
}
