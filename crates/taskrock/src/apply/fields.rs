//! The issue field IDs of an organization.

use std::collections::BTreeMap;

use crate::error::Error;
use crate::gh::Gh;
use crate::github::get_all;

/// The ID of every issue field of `owner`, by name.
///
/// # Errors
///
/// Returns [`Error::Gh`] when `gh` fails, [`Error::Json`] when an entry has
/// no numeric `id` or no string `name`.
pub(super) fn field_ids(gh: &dyn Gh, owner: &str) -> Result<BTreeMap<String, u64>, Error> {
    let mut ids = BTreeMap::new();
    let path = format!("orgs/{owner}/issue-fields?per_page=100");
    for value in get_all(gh, &path)? {
        let name = value["name"]
            .as_str()
            .ok_or_else(|| Error::Json("name".to_owned()))?;
        let id = value["id"]
            .as_u64()
            .ok_or_else(|| Error::Json("id".to_owned()))?;
        ids.insert(name.to_owned(), id);
    }
    Ok(ids)
}
