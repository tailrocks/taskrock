//! The table of codes: name, kind and owner, in rank order.

use super::Code::{self, *};
use crate::pr::status::evaluate::Kind::{self, Blocker, Wait};

const TABLE: [(Code, &str, Kind, &str); 17] = [
    (PrNotOpen, "pr_not_open", Blocker, "human"),
    (UnknownState, "unknown_state", Blocker, "taskrock"),
    (BaseNotDefault, "base_not_default", Blocker, "taskrock"),
    (Conflict, "conflict", Blocker, "builder"),
    (RoundsExhausted, "rounds_exhausted", Blocker, "human"),
    (ThreadsBlocking, "threads_blocking", Blocker, "reviewer"),
    (ThreadsHuman, "threads_human", Blocker, "human"),
    (ThreadsDeferrable, "threads_deferrable", Blocker, "taskrock"),
    (CheckFailed, "check_failed", Blocker, "builder"),
    (VerdictFailed, "verdict_failed", Blocker, "builder"),
    (VerdictMissing, "verdict_missing", Blocker, "reviewer"),
    (Draft, "draft", Blocker, "builder"),
    (ClosesMissing, "closes_missing", Blocker, "builder"),
    (MergeabilityPending, "mergeability_pending", Wait, "github"),
    (BotReviewPending, "bot_review_pending", Wait, "codex"),
    (ChecksPending, "checks_pending", Wait, "ci"),
    (Queued, "queued", Wait, "taskrock"),
];

#[test]
fn evaluate_code_table_pins_name_kind_and_owner_in_rank_order() {
    for (rank, (code, name, kind, owner)) in TABLE.into_iter().enumerate() {
        assert_eq!(code as usize, rank, "{name} has the wrong rank");
        assert_eq!(code.as_str(), name);
        assert_eq!(code.kind(), kind, "{name}");
        assert_eq!(code.owner(), owner, "{name}");
    }
}
