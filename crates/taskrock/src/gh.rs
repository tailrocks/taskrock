//! Something that runs the `gh` command.

mod fake;

pub use fake::FakeGh;

use std::io::Write as _;
use std::process::Stdio;

use crate::error::Error;

#[cfg(test)]
mod tests;

/// Something that runs the `gh` command.
pub trait Gh {
    /// Runs `gh` with these arguments. `input` is written to standard input.
    /// Returns standard output.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Gh`] when `gh` fails.
    fn run(&self, args: &[&str], input: Option<&str>) -> Result<String, Error>;
}

/// Runs the real `gh` program.
#[derive(Debug, Clone)]
pub struct RealGh {
    /// How many times a call is repeated after a rate limit.
    pub retries: u32,
    /// How long to wait before a repeat.
    pub wait: std::time::Duration,
}

impl Default for RealGh {
    fn default() -> Self {
        Self {
            retries: 5,
            wait: std::time::Duration::from_secs(60),
        }
    }
}

impl Gh for RealGh {
    fn run(&self, args: &[&str], input: Option<&str>) -> Result<String, Error> {
        let mut left = self.retries;
        loop {
            let output = run_once(args, input);
            match output {
                Ok(stdout) => return Ok(stdout),
                Err((stderr, stdout)) => {
                    let slow = stderr.to_lowercase();
                    let limited =
                        slow.contains("rate limit") || slow.contains("unknown owner type");
                    if limited && left > 0 {
                        left -= 1;
                        std::thread::sleep(self.wait);
                    } else {
                        return Err(Error::Gh {
                            args: args.join(" "),
                            stderr: stderr.trim().to_owned(),
                            stdout,
                        });
                    }
                }
            }
        }
    }
}

/// One call. A failure is the standard error and the standard output.
fn run_once(args: &[&str], input: Option<&str>) -> Result<String, (String, String)> {
    let failed = |message: String| (message, String::new());
    let mut child = std::process::Command::new("gh")
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| failed(error.to_string()))?;
    if let Some(text) = input {
        child
            .stdin
            .as_mut()
            .ok_or_else(|| failed("gh has no standard input".to_owned()))?
            .write_all(text.as_bytes())
            .map_err(|error| failed(error.to_string()))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| failed(error.to_string()))?;
    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|error| failed(error.to_string()))
    } else {
        Err((
            String::from_utf8_lossy(&output.stderr).into_owned(),
            String::from_utf8_lossy(&output.stdout).into_owned(),
        ))
    }
}
