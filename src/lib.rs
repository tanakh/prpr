//! prpr: a Packrat parser generator.
//!
//! See [`grammar!`] for the grammar language.

pub use prpr_macros::grammar;
pub use prpr_runtime as runtime;
pub use prpr_runtime::{Expected, Found, LineCol, ParseError, Span};
