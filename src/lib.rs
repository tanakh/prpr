//! prpr: a Packrat parser generator.
//!
//! The book in the `book` directory of the repository explains how to write grammars.

pub use prpr_macros::grammar;
pub use prpr_runtime as runtime;
pub use prpr_runtime::{Expected, Found, LineCol, ParseError};

// The examples of the book are tested as documentation tests.
#[cfg(doctest)]
mod book {
    macro_rules! chapters {
        ($($name:ident = $path:literal,)*) => {
            $(
                #[doc = include_str!(concat!("../book/src/", $path))]
                pub struct $name;
            )*
        };
    }

    chapters! {
        Introduction = "introduction.md",
        Installation = "getting-started/installation.md",
        FirstParser = "getting-started/first-parser.md",
        Json = "getting-started/json.md",
        Rules = "guide/rules.md",
        Expressions = "guide/expressions.md",
        ValuesAndActions = "guide/values-and-actions.md",
        Whitespace = "guide/whitespace.md",
        Errors = "guide/errors.md",
        Memoization = "guide/memoization.md",
        BinaryInput = "guide/binary-input.md",
        Composing = "guide/composing.md",
    }
}
