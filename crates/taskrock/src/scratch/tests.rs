//! Tests of `Scratch`.

use std::io::ErrorKind;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

use super::Scratch;

#[test]
fn folder_exists_while_held_and_is_gone_after_drop() {
    let guard = Scratch::new("held").expect("the folder is made");
    let path = guard.path().to_path_buf();
    assert!(path.is_dir());
    drop(guard);
    assert!(!path.exists());
}

#[test]
fn drop_removes_the_content_of_the_folder() {
    let guard = Scratch::new("content").expect("the folder is made");
    let folder = guard.path().to_path_buf();
    let file = guard.write("a.md", "x").expect("the file is written");
    assert_eq!(file, folder.join("a.md"));
    assert_eq!(std::fs::read_to_string(&file).expect("reads back"), "x");
    drop(guard);
    assert!(!file.exists());
    assert!(!folder.exists());
}

#[test]
fn folder_is_removed_when_the_test_panics() {
    let mut seen: Option<(PathBuf, bool)> = None;
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let guard = Scratch::new("panic").expect("the folder is made");
        seen = Some((guard.path().to_path_buf(), guard.path().is_dir()));
        panic!("the test fails");
    }));
    assert!(outcome.is_err());
    let (path, was_dir) = seen.expect("the path was stored");
    assert!(was_dir);
    assert!(!path.exists());
}

#[test]
fn two_folders_with_one_prefix_are_separate() {
    let first = Scratch::new("same").expect("the first folder is made");
    let second = Scratch::new("same").expect("the second folder is made");
    assert_ne!(first.path(), second.path());
    assert!(first.path().is_dir());
    assert!(second.path().is_dir());
    let first_path = first.path().to_path_buf();
    drop(first);
    assert!(second.path().is_dir());
    assert!(!first_path.exists());
}

#[test]
fn new_skips_a_folder_that_already_exists() {
    let parent = Scratch::new("parent").expect("the parent is made");
    let tmp = parent.path().parent().expect("the temporary directory");
    let stale: Vec<PathBuf> = (0..50)
        .map(|k| tmp.join(format!("taskrock-stale-{}-{k}", std::process::id())))
        .collect();
    for path in &stale {
        std::fs::create_dir(path).expect("the stale folder is made");
    }
    let made = Scratch::new("stale");
    for path in &stale {
        std::fs::remove_dir(path).expect("the stale folder is removed");
    }
    let made = made.expect("a free name is found");
    assert!(!stale.iter().any(|path| path == made.path()));
    assert!(made.path().is_dir());
}

#[test]
fn new_returns_an_error_that_is_not_a_name_clash() {
    let error = Scratch::new("no/such/dir").expect_err("the parent folder is missing");
    assert_eq!(error.kind(), ErrorKind::NotFound);
}

#[test]
fn dropping_after_the_folder_was_removed_does_not_panic() {
    let guard = Scratch::new("gone").expect("the folder is made");
    std::fs::remove_dir(guard.path()).expect("the folder is removed by hand");
    drop(guard);
}
