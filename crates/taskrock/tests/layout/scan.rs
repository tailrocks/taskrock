//! A scan of the module declarations of a Rust file, from its code and not from its lines.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) fn file_name(path: &Path) -> &str {
    path.file_name().and_then(|n| n.to_str()).unwrap_or("")
}

/// A word (an identifier or a number) or one other character of Rust code.
#[derive(PartialEq)]
enum Tok {
    Word(String),
    Sym(char),
}

/// The index after a block comment that starts at `i`; block comments nest.
fn skip_block_comment(c: &[char], mut i: usize) -> usize {
    let mut depth = 0;
    while i < c.len() {
        if c[i..].starts_with(&['/', '*']) {
            depth += 1;
            i += 2;
        } else if c[i..].starts_with(&['*', '/']) {
            depth -= 1;
            i += 2;
            if depth == 0 {
                break;
            }
        } else {
            i += 1;
        }
    }
    i
}

/// The index after a string whose opening quote ends just before `i`.
/// `raw` is the number of `#` of a raw string.
fn skip_string(c: &[char], mut i: usize, raw: Option<usize>) -> usize {
    while i < c.len() {
        let hashes = raw.unwrap_or(0);
        let closing = c[i] == '"' && c[i + 1..].iter().take_while(|h| **h == '#').count() >= hashes;
        if closing {
            return i + 1 + hashes;
        }
        i += if c[i] == '\\' && raw.is_none() { 2 } else { 1 };
    }
    i
}

/// The index after the char literal at `i`; a lifetime is one quote.
fn skip_char(c: &[char], i: usize) -> usize {
    if c.get(i + 1) == Some(&'\\') {
        let close = c
            .get(i + 3..)
            .and_then(|rest| rest.iter().position(|ch| *ch == '\''));
        return close.map_or(c.len(), |at| i + 3 + at + 1);
    }
    if c.get(i + 2) == Some(&'\'') {
        i + 3
    } else {
        i + 1
    }
}

/// `(hashes, index after the quote)` when a raw string opens at `i`.
fn raw_string_open(c: &[char], i: usize) -> Option<(usize, usize)> {
    let hashes = c[i..].iter().take_while(|h| **h == '#').count();
    (c.get(i + hashes) == Some(&'"')).then_some((hashes, i + hashes + 1))
}

/// The code of a file as words and symbols, without comments, strings and chars.
fn lex(text: &str) -> Vec<Tok> {
    let c: Vec<char> = text.chars().collect();
    let is_word = |ch: char| ch.is_alphanumeric() || ch == '_';
    let (mut toks, mut i) = (Vec::new(), 0);
    while i < c.len() {
        if c[i].is_whitespace() {
            i += 1;
        } else if c[i..].starts_with(&['/', '/']) {
            i += c[i..].iter().take_while(|ch| **ch != '\n').count();
        } else if c[i..].starts_with(&['/', '*']) {
            i = skip_block_comment(&c, i);
        } else if c[i] == '"' {
            i = skip_string(&c, i + 1, None);
        } else if c[i] == '\'' {
            i = skip_char(&c, i);
        } else if is_word(c[i]) {
            let len = c[i..].iter().take_while(|ch| is_word(**ch)).count();
            let word: String = c[i..i + len].iter().collect();
            i += len;
            match raw_string_open(&c, i).filter(|_| word == "r" || word == "br") {
                Some((hashes, after)) => i = skip_string(&c, after, Some(hashes)),
                None => toks.push(Tok::Word(word)),
            }
        } else {
            toks.push(Tok::Sym(c[i]));
            i += 1;
        }
    }
    toks
}

/// The index of the symbol that closes the group opened at `open`.
fn group_end(toks: &[Tok], open: usize) -> usize {
    let mut depth = 0;
    for (at, tok) in toks.iter().enumerate().skip(open) {
        match tok {
            Tok::Sym('[' | '(') => depth += 1,
            Tok::Sym(']' | ')') => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            return at;
        }
    }
    toks.len()
}

fn attribute_text(toks: &[Tok]) -> String {
    let part = |tok: &Tok| match tok {
        Tok::Word(word) => word.clone(),
        Tok::Sym(sym) => sym.to_string(),
    };
    toks.iter().map(part).collect()
}

/// One `mod name;` of a file, and where the file of the module is.
pub(crate) struct Decl {
    pub(crate) file: PathBuf,
    pub(crate) parent: PathBuf,
    /// The declaration has the attribute `#[cfg(test)]`.
    pub(crate) attr: bool,
    /// The declaration sits in an inline module under `#[cfg(test)]`.
    pub(crate) nested: bool,
    /// The declaration, or an inline module around it, has a `#[path]` attribute.
    pub(crate) path: bool,
    pub(crate) name: String,
}

/// The attributes seen since the last item, which the next `mod` takes.
#[derive(Clone, Copy, Default)]
struct Attrs {
    test: bool,
    path: bool,
}

/// An open `{` of a file: the inline module it opens, or none.
struct Frame {
    name: String,
    attrs: Attrs,
}

/// The attribute text of `#[cfg(test)]`, and whether it is a `path` attribute,
/// also under `cfg_attr`.
fn read_attrs(text: &str) -> Attrs {
    let cfg_attr = text.starts_with("cfg_attr(");
    Attrs {
        test: text == "cfg(test)",
        path: text.starts_with("path=") || (cfg_attr && text.contains(",path=")),
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

/// Every `mod name;` that a file declares, read from its code and not from its lines.
fn declarations(parent: &Path) -> io::Result<Vec<Decl>> {
    let toks = lex(&fs::read_to_string(parent)?);
    let (mut found, mut frames) = (Vec::new(), Vec::<Option<Frame>>::new());
    let (mut attrs, mut i) = (Attrs::default(), 0);
    while i < toks.len() {
        match (&toks[i], toks.get(i + 1), toks.get(i + 2)) {
            (Tok::Sym('#'), next, _) => {
                let open = i + 1 + usize::from(next == Some(&Tok::Sym('!')));
                if toks.get(open) != Some(&Tok::Sym('[')) {
                    attrs = Attrs::default();
                    i += 1;
                    continue;
                }
                let end = group_end(&toks, open);
                if next != Some(&Tok::Sym('!')) {
                    let read = read_attrs(&attribute_text(&toks[open + 1..end]));
                    attrs.test |= read.test;
                    attrs.path |= read.path;
                }
                i = end + 1;
            }
            (Tok::Word(word), Some(Tok::Sym('(')), _) if word == "pub" => {
                i = group_end(&toks, i + 1) + 1;
            }
            (Tok::Word(word), _, _) if word == "pub" => i += 1,
            (Tok::Word(word), Some(Tok::Word(name)), Some(Tok::Sym(end @ (';' | '{'))))
                if word == "mod" =>
            {
                if *end == ';' {
                    let inline: Vec<&str> =
                        frames.iter().flatten().map(|f| f.name.as_str()).collect();
                    found.push(Decl {
                        file: module_file(parent, &inline, name),
                        parent: parent.to_path_buf(),
                        attr: attrs.test,
                        nested: frames.iter().flatten().any(|f| f.attrs.test),
                        path: attrs.path || frames.iter().flatten().any(|f| f.attrs.path),
                        name: name.clone(),
                    });
                } else {
                    frames.push(Some(Frame {
                        name: name.clone(),
                        attrs,
                    }));
                }
                attrs = Attrs::default();
                i += 3;
            }
            (Tok::Sym('{'), _, _) => {
                frames.push(None);
                attrs = Attrs::default();
                i += 1;
            }
            (Tok::Sym('}'), _, _) => {
                frames.pop();
                attrs = Attrs::default();
                i += 1;
            }
            _ => {
                attrs = Attrs::default();
                i += 1;
            }
        }
    }
    Ok(found)
}

pub(crate) fn all_declarations(files: &[PathBuf]) -> io::Result<Vec<Decl>> {
    let mut found = Vec::new();
    for file in files {
        found.extend(declarations(file)?);
    }
    Ok(found)
}
