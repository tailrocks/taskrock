//! The reasons a pull request is not ready, in the order they are reported.

use super::evaluate::Kind;

/// One reason, in rank order: the blockers first, then the waits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Code {
    PrNotOpen,
    UnknownState,
    BaseNotDefault,
    Conflict,
    RoundsExhausted,
    ThreadsBlocking,
    ThreadsHuman,
    ThreadsDeferrable,
    CheckFailed,
    VerdictFailed,
    VerdictMissing,
    Draft,
    ClosesMissing,
    MergeabilityPending,
    BotReviewPending,
    ChecksPending,
    Queued,
}

impl Code {
    /// The name that the output prints.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.row().0
    }

    /// Whether the code stops the merge or only waits.
    #[must_use]
    pub const fn kind(self) -> Kind {
        self.row().1
    }

    /// Who acts on it.
    #[must_use]
    pub const fn owner(self) -> &'static str {
        self.row().2
    }

    const fn row(self) -> (&'static str, Kind, &'static str) {
        use Kind::{Blocker, Wait};
        match self {
            Self::PrNotOpen => ("pr_not_open", Blocker, "human"),
            Self::UnknownState => ("unknown_state", Blocker, "taskrock"),
            Self::BaseNotDefault => ("base_not_default", Blocker, "taskrock"),
            Self::Conflict => ("conflict", Blocker, "builder"),
            Self::RoundsExhausted => ("rounds_exhausted", Blocker, "human"),
            Self::ThreadsBlocking => ("threads_blocking", Blocker, "reviewer"),
            Self::ThreadsHuman => ("threads_human", Blocker, "human"),
            Self::ThreadsDeferrable => ("threads_deferrable", Blocker, "taskrock"),
            Self::CheckFailed => ("check_failed", Blocker, "builder"),
            Self::VerdictFailed => ("verdict_failed", Blocker, "builder"),
            Self::VerdictMissing => ("verdict_missing", Blocker, "reviewer"),
            Self::Draft => ("draft", Blocker, "builder"),
            Self::ClosesMissing => ("closes_missing", Blocker, "builder"),
            Self::MergeabilityPending => ("mergeability_pending", Wait, "github"),
            Self::BotReviewPending => ("bot_review_pending", Wait, "codex"),
            Self::ChecksPending => ("checks_pending", Wait, "ci"),
            Self::Queued => ("queued", Wait, "taskrock"),
        }
    }
}
