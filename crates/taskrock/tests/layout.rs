//! The layout of the unit-test files under `src` against `docs/structure.md`.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const CRATE: &str = env!("CARGO_MANIFEST_DIR");
const DOC: &str = "../../docs/structure.md";

/// A path under `src` as `src/a/b.rs`, with `/` on every platform.
type Rel = String;

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            rust_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

fn rel(path: &Path) -> Rel {
    let crate_root = Path::new(CRATE);
    let tail = path.strip_prefix(crate_root).unwrap_or(path);
    let parts: Vec<_> = tail.iter().map(|part| part.to_string_lossy()).collect();
    parts.join("/")
}

fn sources() -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    rust_files(&Path::new(CRATE).join("src"), &mut files)?;
    files.sort();
    Ok(files)
}

fn doc() -> io::Result<String> {
    fs::read_to_string(Path::new(CRATE).join(DOC))
}

/// A `*_tests.rs` file, or a file directly inside a `tests/` folder.
fn is_listed_kind(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let folder = path.parent().and_then(|p| p.file_name());
    name.ends_with("_tests.rs") || folder.is_some_and(|f| f == "tests")
}

/// A layout that `docs/structure.md` allows for a test-only module.
fn is_allowed_layout(path: &Path) -> bool {
    path.file_name().is_some_and(|n| n == "tests.rs") || is_listed_kind(path)
}

/// The `src/...rs` paths that the page names, in the kind of the lists.
fn documented(text: &str) -> BTreeSet<Rel> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || "_/.-".contains(c)))
        .map(|token| token.trim_end_matches('.'))
        .filter(|token| token.starts_with("src/") && token.ends_with(".rs"))
        .filter(|token| is_listed_kind(Path::new(token)))
        .map(str::to_owned)
        .collect()
}

/// The module name of a line `mod x;` or `pub(...) mod x;`.
fn declared_module(line: &str) -> Option<&str> {
    let mut rest = line.trim();
    if let Some(after) = rest.strip_prefix("pub") {
        rest = match after.strip_prefix('(') {
            Some(inner) => inner.split_once(')')?.1,
            None => after,
        };
    }
    let name = rest.trim().strip_prefix("mod ")?.strip_suffix(';')?;
    Some(name.trim())
}

/// The file of the module `name` that the file `parent` declares.
fn module_file(parent: &Path, name: &str) -> PathBuf {
    let stem = parent.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let dir = parent.parent().unwrap_or(parent);
    let base = match stem {
        "lib" | "main" => dir.to_path_buf(),
        _ => dir.join(stem),
    };
    base.join(format!("{name}.rs"))
}

/// The files of the modules that a file declares only under `#[cfg(test)]`.
fn test_only_modules(parent: &Path) -> io::Result<Vec<PathBuf>> {
    let text = fs::read_to_string(parent)?;
    let mut found = Vec::new();
    let mut pending = false;
    for line in text.lines().map(str::trim) {
        let mut code = line;
        if let Some(rest) = line.strip_prefix("#[cfg(test)]") {
            pending = true;
            code = rest.trim();
        }
        if !pending || code.is_empty() || code.starts_with("//") || code.starts_with("#[") {
            continue;
        }
        pending = false;
        if let Some(name) = declared_module(code) {
            found.push(module_file(parent, name));
        }
    }
    Ok(found)
}

fn fail(problems: &[String]) {
    assert!(problems.is_empty(), "\n{}\n", problems.join("\n"));
}

#[test]
fn every_test_file_is_listed_in_the_structure_page() -> io::Result<()> {
    let listed = documented(&doc()?);
    let mut problems = Vec::new();
    for path in sources()? {
        let name = rel(&path);
        if is_listed_kind(&path) && !listed.contains(&name) {
            problems.push(format!(
                "{name} is a unit-test file that docs/structure.md does not name: \
                 add `{name}` to docs/structure.md"
            ));
        }
    }
    fail(&problems);
    Ok(())
}

#[test]
fn every_listed_test_file_exists() -> io::Result<()> {
    let mut problems = Vec::new();
    for name in documented(&doc()?) {
        if !Path::new(CRATE).join(&name).is_file() {
            problems.push(format!(
                "docs/structure.md names {name}, which does not exist: \
                 remove it from docs/structure.md or restore the file"
            ));
        }
    }
    fail(&problems);
    Ok(())
}

#[test]
fn test_only_modules_use_an_allowed_layout() -> io::Result<()> {
    let mut problems = Vec::new();
    for parent in sources()? {
        for file in test_only_modules(&parent)? {
            if is_allowed_layout(&file) {
                continue;
            }
            let name = file.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let folder = file.parent().unwrap_or(&file).join("tests").join(name);
            problems.push(format!(
                "{} is declared in {} under #[cfg(test)], but it is not a tests.rs, \
                 a *_tests.rs or a file in a tests/ folder: move it to {}",
                rel(&file),
                rel(&parent),
                rel(&folder)
            ));
        }
    }
    fail(&problems);
    Ok(())
}
