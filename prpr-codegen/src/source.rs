//! Where the text of a grammar comes from, and how to get Rust tokens for parts of it.

use crate::ast::Span;
use crate::diag::Diagnostic;
use proc_macro2::{Span as TokenSpan, TokenStream};

/// The text of a grammar, and the Rust tokens of its parts: a
/// [`SourceText`](prpr_runtime::SourceText) for the input of a proc-macro, or a [`TextSource`]
/// for the text of a file.
pub trait Source: SourceImpl {}

impl Source for prpr_runtime::SourceText {}

impl Source for TextSource<'_> {}

/// What the code generator needs from a [`Source`], which only this crate implements.
pub trait SourceImpl {
    /// The text of the grammar.
    fn text(&self) -> &str;

    /// The Rust tokens of `span` of the text.
    fn tokens(&self, span: Span) -> Result<TokenStream, Diagnostic>;

    /// The span to give tokens generated for `span` of the text, so that the compiler reports
    /// problems with them at the grammar.
    fn token_span(&self, span: Span) -> TokenSpan;

    /// Where a token from [`SourceImpl::tokens`] is in the text, if that is known.
    fn locate(&self, span: TokenSpan) -> Option<Span>;

    /// Whether the tokens from [`SourceImpl::tokens`] are where the grammar is in Rust code, so
    /// that problems with tokens that cannot be located are reported by `compile_error!` at the
    /// tokens rather than at the part of the grammar that they come from.
    fn tokens_in_code(&self) -> bool;
}

/// The input of a proc-macro: the original tokens are used for Rust code.
impl SourceImpl for prpr_runtime::SourceText {
    fn text(&self) -> &str {
        self.text()
    }

    fn tokens(&self, span: Span) -> Result<TokenStream, Diagnostic> {
        Ok(self.tokens(span))
    }

    fn token_span(&self, span: Span) -> TokenSpan {
        self.span(span.start)
    }

    fn locate(&self, _span: TokenSpan) -> Option<Span> {
        None
    }

    fn tokens_in_code(&self) -> bool {
        true
    }
}

/// The text of a grammar file, whose Rust code is tokenized from the text.
#[derive(Clone, Copy, Debug)]
pub struct TextSource<'a> {
    text: &'a str,
    span: TokenSpan,
}

impl<'a> TextSource<'a> {
    /// The text of a grammar file.
    pub fn new(text: &'a str) -> Self {
        TextSource {
            text,
            span: TokenSpan::call_site(),
        }
    }

    /// Gives the names generated for the rules `span`, e.g. that of the path of the file in a
    /// proc-macro: a place written by the user, so that the compiler reports problems with them
    /// there and lints them, e.g. reports unused rules with `#![lint]`.
    pub fn with_span(self, span: TokenSpan) -> Self {
        TextSource { span, ..self }
    }
}

impl SourceImpl for TextSource<'_> {
    fn text(&self) -> &str {
        self.text
    }

    fn tokens(&self, span: Span) -> Result<TokenStream, Diagnostic> {
        // Padding puts the tokens at their lines and columns in the grammar, so that they can be
        // located (see `locate`).
        let before = &self.text[..span.start];
        let line_start = before.rfind('\n').map_or(0, |i| i + 1);
        let padded = format!(
            "{}{}{}",
            "\n".repeat(before.matches('\n').count()),
            " ".repeat(before[line_start..].chars().count()),
            &self.text[span]
        );
        padded
            .parse()
            .map_err(|e| Diagnostic::error(span, format!("invalid Rust code: {e}")))
    }

    fn token_span(&self, _span: Span) -> TokenSpan {
        self.span
    }

    fn locate(&self, span: TokenSpan) -> Option<Span> {
        // Generated tokens have empty ranges.
        let len = span.byte_range().len();
        if len == 0 {
            return None;
        }
        let pos = span.start();
        let line_start = match pos.line {
            1 => 0,
            n => self.text.match_indices('\n').nth(n - 2)?.0 + 1,
        };
        let column = self.text[line_start..].char_indices().nth(pos.column)?.0;
        let start = line_start + column;
        Some(Span {
            start,
            end: start + len,
        })
    }

    fn tokens_in_code(&self) -> bool {
        false
    }
}
