//! Tests of the severity parser, on the Codex bodies of the recorded pull requests.

use super::{Severity, parse};

const P2: &str = "**<sub><sub>![P2 Badge](https://img.shields.io/badge/P2-yellow?style=flat)</sub></sub>  Enforce the limit**\n\nText.";
const P1: &str = "**<sub><sub>![P1 Badge](https://img.shields.io/badge/P1-orange?style=flat)</sub></sub>  Run the live test**";

#[test]
fn severity_reads_the_codex_badge() {
    assert_eq!(parse(P2), Some(Severity(2)));
    assert_eq!(parse(P1), Some(Severity(1)));
}

#[test]
fn severity_reads_a_shields_url_without_the_image_text() {
    assert_eq!(
        parse("see https://img.shields.io/badge/P0-red"),
        Some(Severity(0))
    );
}

#[test]
fn severity_takes_the_lowest_when_several_match() {
    let body = "![P3 Badge](x) and ![P1 Badge](y)\nbadge/P2-yellow";
    assert_eq!(parse(body), Some(Severity(1)));
}

#[test]
fn severity_looks_only_at_the_first_five_non_empty_lines() {
    let body = "\n\none\n\ntwo\nthree\nfour\nfive\n![P0 Badge](x)";
    assert_eq!(parse(body), None);
    let body = "\n\none\n\ntwo\nthree\nfour\n![P0 Badge](x)";
    assert_eq!(parse(body), Some(Severity(0)));
}

#[test]
fn severity_ignores_text_that_is_not_a_badge() {
    for body in [
        "",
        "Real, fixed in a commit.",
        "![P Badge](x)",
        "![P2 Bad](x)",
        "badge/P2",
        "![P999 Badge](x)",
    ] {
        assert_eq!(parse(body), None, "{body}");
    }
}

#[test]
fn severity_displays_as_p_and_a_number() {
    assert_eq!(Severity(3).to_string(), "P3");
}
