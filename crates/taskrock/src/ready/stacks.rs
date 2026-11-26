//! The stacked base of each task that `ready --stacked` may list.

use std::collections::{BTreeMap, BTreeSet};

use super::is_candidate;
use crate::error::Error;
use crate::gh::Gh;
use crate::github::items::Item;
use crate::link::IssueLink;
use crate::stack::{self, Stack};

/// The stack of every candidate in `scope` whose open blockers can be a base.
///
/// A candidate that cannot be stacked is skipped, not reported.
///
/// # Errors
///
/// Returns [`Error::Gh`] or [`Error::Json`] when GitHub fails or answers badly.
pub(super) fn stacks(
    gh: &dyn Gh,
    items: &[Item],
    scope: &[&Item],
    claimed: &BTreeSet<IssueLink>,
) -> Result<BTreeMap<IssueLink, Stack>, Error> {
    let mut out = BTreeMap::new();
    for item in scope {
        if !is_stackable(item, claimed) {
            continue;
        }
        match stack::base(gh, items, item) {
            Ok(found) => {
                out.insert(item.link.clone(), found);
            }
            Err(Error::Refused(_)) => {}
            Err(error) => return Err(error),
        }
    }
    Ok(out)
}

fn is_stackable(item: &Item, claimed: &BTreeSet<IssueLink>) -> bool {
    is_candidate(item)
        && !item.routing.is_owner()
        && item.open_blockers > 0
        && item.status.as_deref() == Some("Todo")
        && !claimed.contains(&item.link)
}
