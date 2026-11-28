//! A scan of the module declarations of a Rust file, from its code and not from its lines.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::lex::{Tok, group_end, lex};

pub(crate) fn file_name(path: &Path) -> &str {
    path.file_name().and_then(|n| n.to_str()).unwrap_or("")
}

fn attribute_text(toks: &[Tok]) -> String {
    let part = |tok: &Tok| match tok {
        Tok::Word(word) => word.clone(),
        Tok::Sym(sym) => sym.to_string(),
    };
    toks.iter().map(part).collect()
}

/// One `mod name;` or `mod name { … }` of a file, and where the file of the module is.
pub(crate) struct Decl {
    /// The file of the module; an inline module has none.
    pub(crate) file: Option<PathBuf>,
    pub(crate) parent: PathBuf,
    pub(crate) gate: Gate,
    /// The declaration sits in an inline module under `#[cfg(test)]`.
    pub(crate) nested: bool,
    /// The declaration has a `#[path]` attribute.
    pub(crate) path: bool,
    pub(crate) name: String,
}

/// What the `cfg(…)` attributes of a module say about `test`.
#[derive(Clone, Copy, Default, PartialEq)]
pub(crate) enum Gate {
    /// No `cfg(…)` attribute mentions `test`.
    #[default]
    None,
    /// The attribute `#[cfg(test)]`.
    Test,
    /// A `cfg(…)` attribute that mentions `test` in another form, such as `cfg(all(test))`.
    Other,
}

impl Gate {
    /// The gate of two attributes on one item: `Other` wins, then `Test`.
    fn merge(self, next: Self) -> Self {
        match (self, next) {
            (Self::Other, _) | (_, Self::Other) => Self::Other,
            (Self::Test, _) | (_, Self::Test) => Self::Test,
            _ => Self::None,
        }
    }
}

impl Decl {
    /// The declaration has the attribute `#[cfg(test)]`.
    pub(crate) fn is_cfg_test(&self) -> bool {
        self.gate == Gate::Test
    }
}

/// The attributes seen since the last item, which the next `mod` takes.
#[derive(Clone, Copy, Default)]
struct Attrs {
    gate: Gate,
    path: bool,
}

/// The attribute `#[cfg(test)]`, a `cfg(…)` that mentions `test` in another form, and
/// a `path` attribute, also under `cfg_attr`.
fn read_attrs(toks: &[Tok]) -> Attrs {
    let code = attribute_text(toks);
    let mentions_test = toks.contains(&Tok::Word("test".to_owned()));
    let gate = match code.as_str() {
        "cfg(test)" => Gate::Test,
        _ if code.starts_with("cfg(") && mentions_test => Gate::Other,
        _ => Gate::None,
    };
    let cfg_attr = code.starts_with("cfg_attr(");
    Attrs {
        gate,
        path: code.starts_with("path=") || (cfg_attr && code.contains(",path=")),
    }
}

/// The file of the module `name` that `parent` declares, in the inline modules `inline`.
fn module_file(parent: &Path, inline: &[&str], name: &str) -> PathBuf {
    let dir = parent.parent().unwrap_or(parent);
    let mut path = match file_name(parent).trim_end_matches(".rs") {
        "lib" | "main" | "mod" => dir.to_path_buf(),
        stem => dir.join(stem),
    };
    path.extend(inline);
    path.join(format!("{name}.rs"))
}

/// An open `{` of a file: the inline module it opens, or none.
struct Frame {
    name: String,
    test: bool,
}

/// The state of a scan of one file.
struct Scan<'a> {
    parent: &'a Path,
    frames: Vec<Option<Frame>>,
    attrs: Attrs,
    found: Vec<Decl>,
}

impl Scan<'_> {
    /// Record the module `name`, which has a body when `inline`, and open it.
    fn module(&mut self, name: &str, inline: bool) {
        let modules: Vec<&str> = self
            .frames
            .iter()
            .flatten()
            .map(|f| f.name.as_str())
            .collect();
        let nested = self.frames.iter().flatten().any(|f| f.test);
        self.found.push(Decl {
            file: (!inline).then(|| module_file(self.parent, &modules, name)),
            parent: self.parent.to_path_buf(),
            gate: self.attrs.gate,
            nested,
            path: self.attrs.path,
            name: name.to_owned(),
        });
        if inline {
            let test = self.attrs.gate == Gate::Test;
            self.frames.push(Some(Frame {
                name: name.to_owned(),
                test,
            }));
        }
        self.attrs = Attrs::default();
    }

    /// The attribute at `i`; the index after it.
    fn attribute(&mut self, toks: &[Tok], i: usize) -> usize {
        let inner = toks.get(i + 1) == Some(&Tok::Sym('!'));
        let open = i + 1 + usize::from(inner);
        if toks.get(open) != Some(&Tok::Sym('[')) {
            self.attrs = Attrs::default();
            return i + 1;
        }
        let end = group_end(toks, open);
        if !inner {
            let read = read_attrs(&toks[open + 1..end]);
            self.attrs.gate = self.attrs.gate.merge(read.gate);
            self.attrs.path |= read.path;
        }
        end + 1
    }

    /// The token at `i`; the index of the next one. A macro invocation and the body of
    /// `macro_rules!` are opaque token trees: no `mod` inside them is a module item.
    fn step(&mut self, toks: &[Tok], i: usize) -> usize {
        let open = |at: usize| matches!(toks.get(at), Some(Tok::Sym('(' | '[' | '{')));
        let reset = |scan: &mut Self, next: usize| {
            scan.attrs = Attrs::default();
            next
        };
        match (&toks[i], toks.get(i + 1), toks.get(i + 2)) {
            (Tok::Sym('#'), ..) => self.attribute(toks, i),
            (Tok::Word(word), Some(Tok::Sym('(')), _) if word == "pub" => {
                group_end(toks, i + 1) + 1
            }
            (Tok::Word(word), ..) if word == "pub" => i + 1,
            (Tok::Word(word), Some(Tok::Word(name)), Some(Tok::Sym(end @ (';' | '{'))))
                if word == "mod" =>
            {
                self.module(name, *end == '{');
                i + 3
            }
            (Tok::Word(word), Some(Tok::Sym('!')), Some(Tok::Word(_)))
                if word == "macro_rules" && open(i + 3) =>
            {
                reset(self, group_end(toks, i + 3) + 1)
            }
            (Tok::Word(_), Some(Tok::Sym('!')), _) if open(i + 2) => {
                reset(self, group_end(toks, i + 2) + 1)
            }
            (Tok::Sym('{'), ..) => {
                self.frames.push(None);
                reset(self, i + 1)
            }
            (Tok::Sym('}'), ..) => {
                self.frames.pop();
                reset(self, i + 1)
            }
            _ => reset(self, i + 1),
        }
    }
}

/// Every `mod` item of a file, read from its code and not from its lines.
fn declarations(parent: &Path) -> io::Result<Vec<Decl>> {
    let toks = lex(&fs::read_to_string(parent)?);
    let mut scan = Scan {
        parent,
        frames: Vec::new(),
        attrs: Attrs::default(),
        found: Vec::new(),
    };
    let mut i = 0;
    while i < toks.len() {
        i = scan.step(&toks, i);
    }
    Ok(scan.found)
}

/// The module tree that `src/lib.rs` and `src/main.rs` reach, and the `mod` items in it.
pub(crate) struct Tree {
    pub(crate) files: Vec<PathBuf>,
    pub(crate) decls: Vec<Decl>,
}

pub(crate) fn module_tree(src: &Path) -> io::Result<Tree> {
    let roots = ["lib.rs", "main.rs"].map(|name| src.join(name));
    let mut todo: Vec<PathBuf> = roots.into_iter().filter(|root| root.is_file()).collect();
    let (mut files, mut decls) = (BTreeSet::new(), Vec::new());
    while let Some(file) = todo.pop() {
        if files.insert(file.clone()) {
            let found = declarations(&file)?;
            todo.extend(
                found
                    .iter()
                    .filter_map(|d| d.file.clone())
                    .filter(|f| f.is_file()),
            );
            decls.extend(found);
        }
    }
    Ok(Tree {
        files: files.into_iter().collect(),
        decls,
    })
}
