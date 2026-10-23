#![cfg(feature = "proc-macro2")]

use proc_macro2::TokenStream;
use prpr_runtime::{Expected, Found, MacroError, ParseError, SourceText, span};

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
fn spans_ranges() {
    let src = source_text("a bc d");
    let column = |span: proc_macro2::Span| span.start().column;
    let (start, end) = src.span_range(span(0, 4));
    assert_eq!((column(start), column(end)), (0, 2));
    // An empty range is at the token where it is.
    let (start, end) = src.span_range(span(5, 5));
    assert_eq!((column(start), column(end)), (5, 5));
}

#[test]
fn reports_errors_of_expansions() {
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

#[test]
fn converts_parse_errors() {
    let e = ParseError::new(2, vec![Expected::Literal("x")], Found::Char('y'));
    let error = MacroError::from(e.clone());
    assert_eq!(error, MacroError::new(e.span(), "expected `x`, found `y`"));
}
