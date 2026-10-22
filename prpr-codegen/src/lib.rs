//! The code generator of prpr, a Packrat parser generator.
//!
//! [`compile`] turns a grammar into Rust tokens; the `grammar!` macro and the `prpr` command
//! are thin wrappers around it.

pub mod analysis;
pub mod ast;
mod codegen;
pub mod diag;
#[cfg(feature = "pretty")]
pub mod file;
pub mod parser;
pub mod source;

pub use codegen::Options;
pub use diag::Diagnostic;
pub use source::{Source, TextSource};

use proc_macro2::TokenStream;

/// Compiles a grammar into Rust items.
pub fn compile(src: &dyn Source, opts: &Options) -> Result<TokenStream, Vec<Diagnostic>> {
    let grammar = parser::parse(src.text()).map_err(|d| vec![d])?;
    let analysis = analysis::analyze(&grammar, src.text())?;
    codegen::generate(src, &analysis, opts)
}
