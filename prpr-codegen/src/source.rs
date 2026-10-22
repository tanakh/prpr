//! Where the text of a grammar comes from, and how to get Rust tokens for parts of it.

use crate::ast::Span;
use crate::diag::Diagnostic;
use proc_macro2::{Span as TokenSpan, TokenStream};

pub trait Source {
    /// The text of the grammar.
    fn text(&self) -> &str;

    /// The Rust tokens of `span` of the text.
    fn tokens(&self, span: Span) -> Result<TokenStream, Diagnostic>;

    /// The span to give tokens generated for `span` of the text, so that the compiler reports
    /// problems with them at the grammar.
    fn token_span(&self, span: Span) -> TokenSpan;
}

/// The input of a proc-macro: the original tokens are used for Rust code.
impl Source for prpr_runtime::SourceText {
    fn text(&self) -> &str {
        self.text()
    }

    fn tokens(&self, span: Span) -> Result<TokenStream, Diagnostic> {
        Ok(self.tokens(span))
    }

    fn token_span(&self, span: Span) -> TokenSpan {
        self.span(span.start)
    }
}

/// A grammar file: Rust code is tokenized from the text.
pub struct TextSource<'a>(pub &'a str);

impl Source for TextSource<'_> {
    fn text(&self) -> &str {
        self.0
    }

    fn tokens(&self, span: Span) -> Result<TokenStream, Diagnostic> {
        // Padding puts the tokens at their lines and columns in the grammar, so that errors in
        // them can be located (see `TextSource::offset`).
        let before = &self.0[..span.start];
        let line_start = before.rfind('\n').map_or(0, |i| i + 1);
        let padded = format!(
            "{}{}{}",
            "\n".repeat(before.matches('\n').count()),
            " ".repeat(before[line_start..].chars().count()),
            &self.0[span]
        );
        padded
            .parse()
            .map_err(|e| Diagnostic::error(span, format!("invalid Rust code: {e}")))
    }

    fn token_span(&self, _span: Span) -> TokenSpan {
        TokenSpan::call_site()
    }
}

impl TextSource<'_> {
    /// The offset in the text of a token from [`Source::tokens`], or `None` for generated tokens.
    pub fn offset(&self, span: TokenSpan) -> Option<usize> {
        if span.byte_range().is_empty() {
            return None;
        }
        let pos = span.start();
        let line_start = match pos.line {
            1 => 0,
            n => self.0.match_indices('\n').nth(n - 2)?.0 + 1,
        };
        let column = self.0[line_start..].char_indices().nth(pos.column)?.0;
        Some(line_start + column)
    }
}
