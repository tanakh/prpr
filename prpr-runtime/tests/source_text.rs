#![cfg(feature = "proc-macro2")]

use proc_macro2::TokenStream;
use prpr_runtime::{SourceText, span};

fn source_text(s: &str) -> SourceText {
    SourceText::new(s.parse::<TokenStream>().unwrap())
}

#[test]
fn rebuilds_the_layout() {
    // Comments become spaces; everything else is as written.
    let src = source_text("a-zA-Z0-9_ ( 'x' \"s\" ) // comment\n  b::<'i>");
    assert_eq!(src.text(), "a-zA-Z0-9_ ( 'x' \"s\" )\n  b::<'i>");
}

#[test]
fn maps_offsets_to_tokens() {
    let src = source_text("foo { bar(1) } baz");
    let text = src.text();
    let at = |s: &str| text.find(s).unwrap();
    // The whole group, and a part of it.
    let group = span(at("{"), at("}") + 1);
    assert_eq!(src.tokens(group).to_string(), "{ bar (1) }");
    let inner = span(at("bar"), at("baz"));
    assert_eq!(src.tokens(inner).to_string(), "bar (1)");
    assert_eq!(src.span(at("baz")).start().column, at("baz"));
    // Between tokens, the next token.
    assert_eq!(src.span(at("{") - 1).start().column, at("{"));
}

#[test]
fn reports_errors_of_expansions() {
    use prpr_runtime::MacroError;
    let input: TokenStream = "1 + x".parse().unwrap();
    let ok = SourceText::with(input.clone(), |src| Ok(src.text().parse().unwrap()));
    assert_eq!(ok.to_string(), "1 + x");
    let error = SourceText::with(input, |src| {
        let at = src.text().find('x').unwrap();
        Err(MacroError::new(span(at, at + 1), "unknown variable"))
    });
    assert!(error.to_string().contains("compile_error"));
    assert!(error.to_string().contains("\"unknown variable\""));
}
