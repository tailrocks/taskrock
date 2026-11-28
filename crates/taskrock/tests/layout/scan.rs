//! A scan of the module declarations of a Rust file, read with `syn`: attributes, inline
//! modules and raw identifiers come from the parser, `syn::visit` reaches every `mod` item
//! (also in a function body, an `impl` block or a `const` block), and a macro body is an
//! opaque token stream that holds no module item.

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use syn::ext::IdentExt;
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::{Attribute, ItemMod, Meta, Token};

pub(crate) fn file_name(path: &Path) -> &str {
    path.file_name().and_then(|n| n.to_str()).unwrap_or("")
}

/// One `mod name;` or `mod name { … }` of a file, and where the file of the module is.
pub(crate) struct Decl {
    /// The file of the module; an inline module has none.
    pub(crate) file: Option<PathBuf>,
    pub(crate) parent: PathBuf,
    pub(crate) gate: Gate,
    /// The declaration sits in an inline module under `#[cfg(test)]`.
    pub(crate) nested: bool,
    /// The declaration has a `#[path]` attribute, also one that a `cfg_attr` applies.
    pub(crate) path: bool,
    /// A `cfg_attr` attribute applies a `cfg` attribute, which hides it from the gate.
    pub(crate) cfg_attr: bool,
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

    /// The gate of the predicate of one `cfg(…)`.
    fn of(predicate: &Meta) -> syn::Result<Self> {
        Ok(if is_test(predicate) {
            Self::Test
        } else if mentions_test(predicate)? {
            Self::Other
        } else {
            Self::None
        })
    }
}

impl Decl {
    /// The declaration has the attribute `#[cfg(test)]`.
    pub(crate) fn is_cfg_test(&self) -> bool {
        self.gate == Gate::Test
    }
}

/// The name of an attribute or predicate without a path and without the raw marker.
fn name(meta: &Meta) -> Option<String> {
    meta.path().get_ident().map(|id| id.unraw().to_string())
}

fn is_test(meta: &Meta) -> bool {
    matches!(meta, Meta::Path(_)) && name(meta).is_some_and(|n| n == "test")
}

/// The predicate is, or contains, the word `test`: `all(test)`, `not(test)`, …
fn mentions_test(predicate: &Meta) -> syn::Result<bool> {
    match predicate {
        Meta::List(list) => {
            let inner = list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
            let mentions: syn::Result<Vec<bool>> = inner.iter().map(mentions_test).collect();
            Ok(mentions?.contains(&true))
        }
        other => Ok(is_test(other)),
    }
}

/// The attributes of a module that the checks read.
#[derive(Clone, Copy, Default)]
struct Attrs {
    gate: Gate,
    path: bool,
    cfg_attr: bool,
}

impl Attrs {
    /// Every attribute of an item, outer and inner, in a `cfg_attr` or not.
    fn of(attrs: &[Attribute]) -> syn::Result<Self> {
        let mut read = Self::default();
        for attr in attrs {
            read.add(&attr.meta, false)?;
        }
        Ok(read)
    }

    /// One attribute. The attributes that a `cfg_attr` applies are `conditional`: they are
    /// inspected, a nested `cfg` is refused, and `path` counts as `path`.
    fn add(&mut self, meta: &Meta, conditional: bool) -> syn::Result<()> {
        match (name(meta).as_deref(), conditional) {
            (Some("path"), _) => self.path = true,
            (Some("cfg"), true) => self.cfg_attr = true,
            (Some("cfg"), false) => {
                let predicate = meta.require_list()?.parse_args::<Meta>()?;
                self.gate = self.gate.merge(Gate::of(&predicate)?);
            }
            (Some("cfg_attr"), _) => {
                let args = meta
                    .require_list()?
                    .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
                for nested in args.iter().skip(1) {
                    self.add(nested, true)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// The file of the module `name` that `parent` declares, in the inline modules `inline`.
fn module_file(parent: &Path, inline: &[String], name: &str) -> PathBuf {
    let dir = parent.parent().unwrap_or(parent);
    let mut path = match file_name(parent).trim_end_matches(".rs") {
        "lib" | "main" | "mod" => dir.to_path_buf(),
        stem => dir.join(stem),
    };
    path.extend(inline);
    path.join(format!("{name}.rs"))
}

/// The state of a scan of one file. It visits every `mod` item, wherever it sits: at the top
/// level, in an inline module, in a function body, in an `impl` block or in a `const` block.
/// A block does not change the folder of the file of a module that it declares.
struct Scan<'a> {
    parent: &'a Path,
    /// The names of the inline modules that the scan is in.
    inline: Vec<String>,
    /// The scan is in an inline module under `#[cfg(test)]`.
    nested: bool,
    found: Vec<Decl>,
    /// The first attribute that did not parse.
    error: Option<syn::Error>,
}

impl<'ast> Visit<'ast> for Scan<'_> {
    fn visit_item_mod(&mut self, module: &'ast ItemMod) {
        let attrs = match Attrs::of(&module.attrs) {
            Ok(attrs) => attrs,
            Err(err) => {
                self.error.get_or_insert(err);
                return;
            }
        };
        let name = module.ident.unraw().to_string();
        let inline = module.content.is_some();
        self.found.push(Decl {
            file: (!inline).then(|| module_file(self.parent, &self.inline, &name)),
            parent: self.parent.to_path_buf(),
            gate: attrs.gate,
            nested: self.nested,
            path: attrs.path,
            cfg_attr: attrs.cfg_attr,
            name: name.clone(),
        });
        let was_nested = self.nested;
        self.nested |= attrs.gate == Gate::Test;
        self.inline.push(name);
        visit::visit_item_mod(self, module);
        self.inline.pop();
        self.nested = was_nested;
    }
}

/// The inner attributes of a file and every `mod` item of it.
fn read(parent: &Path) -> io::Result<(Attrs, Vec<Decl>)> {
    let invalid = |err: syn::Error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: {err}", parent.display()),
        )
    };
    let file = syn::parse_file(&fs::read_to_string(parent)?).map_err(invalid)?;
    let inner = Attrs::of(&file.attrs).map_err(invalid)?;
    let mut scan = Scan {
        parent,
        inline: Vec::new(),
        nested: inner.gate == Gate::Test,
        found: Vec::new(),
        error: None,
    };
    scan.visit_file(&file);
    scan.error
        .map_or(Ok((inner, scan.found)), |err| Err(invalid(err)))
}

/// Every `mod` item of a file, in its inline modules, function bodies and blocks too.
pub(crate) fn declarations(parent: &Path) -> io::Result<Vec<Decl>> {
    Ok(read(parent)?.1)
}

/// The file opens with a `#![path]` attribute, also one that a `cfg_attr` applies.
pub(crate) fn has_inner_path(parent: &Path) -> io::Result<bool> {
    Ok(read(parent)?.0.path)
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
    let mut inners = BTreeMap::new();
    while let Some(file) = todo.pop() {
        if files.insert(file.clone()) {
            let (inner, found) = read(&file)?;
            todo.extend(
                found
                    .iter()
                    .filter_map(|d| d.file.clone())
                    .filter(|f| f.is_file()),
            );
            decls.extend(found);
            inners.insert(file, inner);
        }
    }
    // A file that opens with `#![cfg(test)]` is gated like a `#[cfg(test)]` on its `mod`.
    for decl in &mut decls {
        if let Some(inner) = decl.file.as_ref().and_then(|file| inners.get(file)) {
            decl.gate = decl.gate.merge(inner.gate);
            decl.cfg_attr |= inner.cfg_attr;
        }
    }
    Ok(Tree {
        files: files.into_iter().collect(),
        decls,
    })
}
