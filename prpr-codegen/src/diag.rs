//! Diagnostics about grammars.

use crate::ast::Span;
use std::fmt;

/// How serious a diagnostic is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Level {
    /// No code can be generated.
    Error,
}

/// A problem with a grammar, at a byte range of its text.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Diagnostic {
    /// How serious the problem is.
    pub level: Level,
    /// Where the problem is.
    pub span: Span,
    /// What the problem is.
    pub message: String,
    /// Further information.
    pub notes: Vec<Note>,
}

/// Additional information, possibly about another part of the grammar.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Note {
    /// The part of the grammar that the note is about, if any.
    pub span: Option<Span>,
    /// The note.
    pub message: String,
}

impl Diagnostic {
    /// An error at `span`.
    pub fn error(span: Span, message: impl Into<String>) -> Self {
        Self {
            level: Level::Error,
            span,
            message: message.into(),
            notes: Vec::new(),
        }
    }

    /// Adds a note.
    pub fn note(mut self, message: impl Into<String>) -> Self {
        self.notes.push(Note {
            span: None,
            message: message.into(),
        });
        self
    }

    /// Adds a note about `span`.
    pub fn note_at(mut self, span: Span, message: impl Into<String>) -> Self {
        self.notes.push(Note {
            span: Some(span),
            message: message.into(),
        });
        self
    }

    /// Whether the diagnostic is an error.
    pub fn is_error(&self) -> bool {
        self.level == Level::Error
    }

    /// Renders the diagnostic with the offending lines of `src`, in the style of rustc.
    pub fn render(&self, src: &str, origin: &str) -> String {
        let level = match self.level {
            Level::Error => "error",
        };
        let snippet = |level, message: &String, span| {
            prpr_runtime::Snippet {
                level,
                message,
                src,
                origin,
                span,
            }
            .to_string()
        };
        let mut out = snippet(level, &self.message, self.span);
        for note in &self.notes {
            match note.span {
                Some(span) => {
                    out.push('\n');
                    out.push_str(&snippet("note", &note.message, span));
                }
                None => {
                    out.push_str("\n  = note: ");
                    out.push_str(&note.message);
                }
            }
        }
        out
    }

    /// The message followed by the notes, as plain text. `locate` describes where a note's span
    /// is, e.g. "line 3".
    pub fn to_text(&self, locate: impl Fn(Span) -> String) -> String {
        let mut out = self.message.clone();
        for note in &self.notes {
            out.push_str("\n= note: ");
            out.push_str(&note.message);
            if let Some(span) = note.span {
                out.push_str(", at ");
                out.push_str(&locate(span));
            }
        }
        out
    }
}

/// The message followed by the notes, without the lines of the grammar that they are about.
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)?;
        for note in &self.notes {
            write!(f, "\n= note: {}", note.message)?;
        }
        Ok(())
    }
}

impl std::error::Error for Diagnostic {}
