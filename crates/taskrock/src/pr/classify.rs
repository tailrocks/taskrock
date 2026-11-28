//! Who must act on a review thread.

use super::severity::{self, Severity};
use super::snapshot::ThreadFact;

#[cfg(test)]
mod tests;

/// What a thread asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// A bot finding that blocks the merge until the reviewer rules on it.
    Blocking,
    /// A P3 bot finding: resolve it with a reply.
    Deferrable,
    /// A thread that a person started.
    Human,
}

impl Class {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Blocking => "blocking",
            Self::Deferrable => "deferrable",
            Self::Human => "human",
        }
    }
}

/// The class of one thread, from its first comment.
#[must_use]
pub fn classify(thread: &ThreadFact) -> Class {
    let bot = thread.first_is_bot || thread.first_author.to_lowercase().ends_with("[bot]");
    match (bot, severity::parse(&thread.first_body)) {
        (false, _) => Class::Human,
        (true, Some(Severity(3))) => Class::Deferrable,
        (true, _) => Class::Blocking,
    }
}
