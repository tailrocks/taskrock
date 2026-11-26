//! The issues that already exist and that the draft links to.

use std::collections::BTreeMap;

use crate::apply::Created;
use crate::draft::{Draft, links};
use crate::error::Error;
use crate::gh::Gh;
use crate::github::{get, is_status};

#[cfg(test)]
mod flow_tests;
#[cfg(test)]
mod tests;

/// Reads each issue the draft uses as `parent` or in `blocked_by` by link.
/// The map key is the link text, which is never a key of the draft.
///
/// # Errors
///
/// Returns [`Error::Apply`] when a link points at no issue or at a pull
/// request, [`Error::Gh`] when `gh` fails, [`Error::Json`] when the answer
/// has no numeric `id`.
pub(super) fn resolve(gh: &dyn Gh, draft: &Draft) -> Result<BTreeMap<String, Created>, Error> {
    let mut resolved = BTreeMap::new();
    for link in links::used(draft) {
        let value = match get(gh, &link.api_path()) {
            Ok(value) => value,
            Err(error) if is_status(&error, 404) || is_status(&error, 410) => {
                return Err(Error::Apply(format!("no issue {}", link.url())));
            }
            Err(error) => return Err(error),
        };
        if value.get("pull_request").is_some() {
            return Err(Error::Apply(format!(
                "{} is a pull request, not an issue",
                link.url()
            )));
        }
        let id = value["id"]
            .as_u64()
            .ok_or_else(|| Error::Json("id".to_owned()))?;
        resolved.insert(
            link.url(),
            Created {
                link,
                id,
                fresh: false,
            },
        );
    }
    Ok(resolved)
}
