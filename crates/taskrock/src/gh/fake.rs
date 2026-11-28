//! A scripted `Gh` for tests.

use std::cell::RefCell;

use crate::error::Error;
use crate::gh::Gh;

/// A scripted `Gh` for tests.
#[derive(Debug, Default)]
pub struct FakeGh {
    replies: RefCell<Vec<Reply>>,
    calls: RefCell<Vec<(String, Option<String>)>>,
}

#[derive(Debug)]
struct Reply {
    when: String,
    stdout: Result<String, (String, String)>,
    used: bool,
}

impl FakeGh {
    /// A fake with no replies.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a reply. It answers the first call whose arguments, joined by
    /// one space, contain `when`. A reply is used once.
    #[must_use]
    pub fn ok(self, when: &str, stdout: &str) -> Self {
        self.push(when, Ok(stdout.to_owned()));
        self
    }

    /// Adds a failing reply. The call returns `Error::Gh` with this `stderr`.
    #[must_use]
    pub fn fail(self, when: &str, stderr: &str) -> Self {
        self.push(when, Err((stderr.to_owned(), String::new())));
        self
    }

    /// Adds a failing reply that printed `stdout` before it failed, as the real `gh` does
    /// for a GraphQL answer with `errors`.
    #[must_use]
    pub fn fail_with_output(self, when: &str, stderr: &str, stdout: &str) -> Self {
        self.push(when, Err((stderr.to_owned(), stdout.to_owned())));
        self
    }

    /// Every call so far: the joined arguments and the input.
    #[must_use]
    pub fn calls(&self) -> Vec<(String, Option<String>)> {
        self.calls.borrow().clone()
    }

    fn push(&self, when: &str, stdout: Result<String, (String, String)>) {
        self.replies.borrow_mut().push(Reply {
            when: when.to_owned(),
            stdout,
            used: false,
        });
    }
}

impl Gh for FakeGh {
    fn run(&self, args: &[&str], input: Option<&str>) -> Result<String, Error> {
        let joined = args.join(" ");
        self.calls
            .borrow_mut()
            .push((joined.clone(), input.map(str::to_owned)));
        let mut replies = self.replies.borrow_mut();
        let found = replies
            .iter_mut()
            .find(|reply| !reply.used && joined.contains(&reply.when));
        match found {
            Some(reply) => {
                reply.used = true;
                match reply.stdout.clone() {
                    Ok(stdout) => Ok(stdout),
                    Err((stderr, stdout)) => Err(Error::Gh {
                        args: joined,
                        stderr,
                        stdout,
                    }),
                }
            }
            None => Err(Error::Gh {
                args: joined.clone(),
                stderr: format!("no fake reply for: {joined}"),
                stdout: String::new(),
            }),
        }
    }
}
