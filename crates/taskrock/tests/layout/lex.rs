//! The code of a Rust file as words and symbols, without comments, strings and chars.

/// A word (an identifier or a number) or one other character of Rust code.
#[derive(PartialEq)]
pub(crate) enum Tok {
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

fn is_word(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

/// The word that starts at `i` and the index after it. A raw identifier `r#name`
/// is the word `name`, and a raw string is no word: it is skipped.
fn word_or_skip(c: &[char], mut i: usize) -> (Option<String>, usize) {
    let len = c[i..].iter().take_while(|ch| is_word(**ch)).count();
    let word: String = c[i..i + len].iter().collect();
    i += len;
    if let Some((hashes, after)) = raw_string_open(c, i).filter(|_| word == "r" || word == "br") {
        return (None, skip_string(c, after, Some(hashes)));
    }
    if word == "r" && c.get(i) == Some(&'#') && c.get(i + 1).is_some_and(|ch| is_word(*ch)) {
        return word_or_skip(c, i + 1);
    }
    (Some(word), i)
}

/// The code of a file as words and symbols, without comments, strings and chars.
pub(crate) fn lex(text: &str) -> Vec<Tok> {
    let c: Vec<char> = text.chars().collect();
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
            let (word, after) = word_or_skip(&c, i);
            toks.extend(word.map(Tok::Word));
            i = after;
        } else {
            toks.push(Tok::Sym(c[i]));
            i += 1;
        }
    }
    toks
}

/// The index of the symbol that closes the group opened at `open`.
pub(crate) fn group_end(toks: &[Tok], open: usize) -> usize {
    let mut depth = 0;
    for (at, tok) in toks.iter().enumerate().skip(open) {
        match tok {
            Tok::Sym('[' | '(' | '{') => depth += 1,
            Tok::Sym(']' | ')' | '}') => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            return at;
        }
    }
    toks.len()
}
