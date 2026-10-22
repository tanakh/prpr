//! Diagnostics about grammars.

use crate::ast::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub level: Level,
    pub span: Span,
    pub message: String,
    pub notes: Vec<Note>,
}

/// Additional information, possibly about another part of the grammar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub span: Option<Span>,
    pub message: String,
}

impl Diagnostic {
    pub fn error(span: Span, message: impl Into<String>) -> Self {
        Self {
            level: Level::Error,
            span,
            message: message.into(),
            notes: Vec::new(),
        }
    }

    pub fn warning(span: Span, message: impl Into<String>) -> Self {
        Self {
            level: Level::Warning,
            ..Self::error(span, message)
        }
    }

    pub fn note(mut self, message: impl Into<String>) -> Self {
        self.notes.push(Note {
            span: None,
            message: message.into(),
        });
        self
    }

    pub fn note_at(mut self, span: Span, message: impl Into<String>) -> Self {
        self.notes.push(Note {
            span: Some(span),
            message: message.into(),
        });
        self
    }

    pub fn is_error(&self) -> bool {
        self.level == Level::Error
    }

    /// Renders the diagnostic with the offending lines of `src`, in the style of rustc.
    pub fn render(&self, src: &str, origin: &str) -> String {
        let level = match self.level {
            Level::Error => "error",
            Level::Warning => "warning",
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
