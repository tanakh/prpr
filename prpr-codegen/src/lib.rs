#![doc = include_str!("../README.md")]

mod analysis;
mod ast;
#[cfg(feature = "build")]
pub mod build;
mod codegen;
mod diag;
#[cfg(feature = "build")]
pub mod file;
mod lint;
mod parser;
mod source;

pub use codegen::Options;
pub use diag::{Diagnostic, Level, Note};
pub use source::{Source, TextSource};

use proc_macro2::TokenStream;

/// Compiles a grammar into Rust items.
pub fn compile(src: &dyn Source, opts: &Options) -> Result<TokenStream, Vec<Diagnostic>> {
    let grammar = parser::parse(src.text()).map_err(|d| vec![d])?;
    let analysis = analysis::analyze(&grammar, src.text())?;
    codegen::generate(src, &analysis, opts)
}
