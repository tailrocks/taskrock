//! Temporary folders that remove themselves.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(test)]
mod tests;

/// The number of the next folder of this process.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// A temporary folder that removes itself, with its content, when it is dropped.
#[derive(Debug)]
pub struct Scratch {
    path: PathBuf,
}

impl Scratch {
    /// Makes the folder `taskrock-<prefix>-<process id>-<n>` in the temporary directory.
    ///
    /// A folder of that name that an old run left is skipped: the next `n` is tried.
    ///
    /// # Errors
    ///
    /// Returns the error of `std::fs::create_dir`, except for a name that is already taken.
    pub fn new(prefix: &str) -> std::io::Result<Self> {
        let tmp = std::env::temp_dir();
        loop {
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = tmp.join(format!("taskrock-{prefix}-{}-{n}", std::process::id()));
            match std::fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
    }

    /// The folder.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Writes `text` to the file `name` of the folder and returns the path of the file.
    ///
    /// # Errors
    ///
    /// Returns the error of `std::fs::write`.
    pub fn write(&self, name: &str, text: &str) -> std::io::Result<PathBuf> {
        let path = self.path.join(name);
        std::fs::write(&path, text)?;
        Ok(path)
    }
}

impl Drop for Scratch {
    /// Removes the folder. A folder that is already gone is fine: a test may remove it itself.
    fn drop(&mut self) {
        match std::fs::remove_dir_all(&self.path) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => eprintln!("taskrock: cannot remove {}: {error}", self.path.display()),
        }
    }
}
