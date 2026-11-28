//! The layout of the unit-test files under `src` against `docs/structure.md`.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[path = "layout/scan.rs"]
mod scan;
use scan::{Decl, all_declarations, file_name};

const CRATE: &str = env!("CARGO_MANIFEST_DIR");
const DOC: &str = "../../docs/structure.md";
const CRATE_PREFIX: &str = "crates/taskrock/";

/// A path under the crate as `src/a/b.rs`, with `/` on every platform.
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

/// The path below the crate root, so a folder above the checkout never counts.
fn local(path: &Path) -> &Path {
    path.strip_prefix(CRATE).unwrap_or(path)
}

fn rel(path: &Path) -> Rel {
    let parts: Vec<_> = local(path)
        .iter()
        .map(|part| part.to_string_lossy())
        .collect();
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
    let folder = path.parent().and_then(|p| p.file_name());
    file_name(path).ends_with("_tests.rs") || folder.is_some_and(|f| f == "tests")
}

/// How many `tests/` folders the path sits in.
fn tests_depth(path: &Path) -> usize {
    let dirs = local(path).parent().into_iter().flat_map(Path::iter);
    dirs.filter(|dir| *dir == "tests").count()
}

/// A layout that `docs/structure.md` allows for test-only code: a `tests.rs` or a
/// `*_tests.rs` outside any `tests/` folder, or a file directly in one `tests/` folder.
fn is_allowed_layout(path: &Path) -> bool {
    let name = file_name(path);
    let tests_file = name == "tests.rs" || name.ends_with("_tests.rs");
    let in_tests_folder = path.parent().is_some_and(|p| file_name(p) == "tests");
    match tests_depth(path) {
        0 => tests_file,
        1 => in_tests_folder,
        _ => false,
    }
}

/// Where a misplaced test-only file goes: directly into its nearest `tests/` folder.
fn suggest(file: &Path) -> Rel {
    let local = local(file);
    let nearest = local
        .ancestors()
        .skip(1)
        .find(|dir| file_name(dir) == "tests");
    let dir = nearest.map_or_else(|| local.with_file_name("tests"), Path::to_path_buf);
    rel(&dir.join(file_name(file)))
}

/// The `src/...rs` paths that the page names, in the kind of the lists. A path
/// that starts with `crates/taskrock/` counts as the path below it.
fn documented(text: &str) -> BTreeSet<Rel> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || "_/.-".contains(c)))
        .map(|token| token.trim_end_matches('.'))
        .map(|token| token.strip_prefix(CRATE_PREFIX).unwrap_or(token))
        .filter(|token| {
            token.starts_with("src/") && Path::new(token).extension().is_some_and(|e| e == "rs")
        })
        .filter(|token| is_listed_kind(Path::new(token)))
        .map(str::to_owned)
        .collect()
}

/// Every file of test-only code: a module declared under `#[cfg(test)]` or in an
/// inline module under it, a file of a `tests/` folder, and each module they declare.
fn test_only_files(decls: &[Decl], files: &[PathBuf]) -> BTreeSet<PathBuf> {
    let mut only: BTreeSet<PathBuf> = files
        .iter()
        .filter(|f| tests_depth(f) > 0)
        .cloned()
        .collect();
    only.extend(
        decls
            .iter()
            .filter(|d| d.attr || d.nested)
            .map(|d| d.file.clone()),
    );
    loop {
        let before = only.len();
        let below: Vec<_> = decls.iter().filter(|d| only.contains(&d.parent)).collect();
        only.extend(below.into_iter().map(|d| d.file.clone()));
        if only.len() == before {
            return only;
        }
    }
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
    for decl in all_declarations(&sources()?)? {
        if decl.attr && !decl.nested && !is_allowed_layout(&decl.file) {
            problems.push(format!(
                "{} is declared in {} under #[cfg(test)], but it is not a tests.rs, \
                 a *_tests.rs or a file in a tests/ folder: move it to {}",
                rel(&decl.file),
                rel(&decl.parent),
                suggest(&decl.file)
            ));
        }
    }
    fail(&problems);
    Ok(())
}

#[test]
fn descendants_of_test_only_modules_use_an_allowed_layout() -> io::Result<()> {
    let files = sources()?;
    let decls = all_declarations(&files)?;
    let direct: BTreeSet<&Path> = decls
        .iter()
        .filter(|d| d.attr && !d.nested)
        .map(|d| d.file.as_path())
        .collect();
    let mut problems = Vec::new();
    for file in test_only_files(&decls, &files) {
        if files.contains(&file) && !direct.contains(file.as_path()) && !is_allowed_layout(&file) {
            problems.push(format!(
                "{} is part of a test-only module, but it is not a tests.rs or a *_tests.rs \
                 outside a tests/ folder, or a file directly in a tests/ folder: move it to {}",
                rel(&file),
                suggest(&file)
            ));
        }
    }
    fail(&problems);
    Ok(())
}
