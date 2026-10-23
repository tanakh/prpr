#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;
#[cfg(feature = "proc-macro2")]
extern crate std;

mod rt;
pub use rt::*;

#[cfg(feature = "proc-macro2")]
mod source_text;
#[cfg(feature = "proc-macro2")]
pub use source_text::{MacroError, SourceText};

/// Includes the module that `prpr_codegen::build` generated in a build script from the grammar
/// file at `path`, given as in the build script.
///
/// ```ignore
/// mod list {
///     prpr_runtime::include_parser!("src/list.peg");
/// }
/// ```
#[macro_export]
macro_rules! include_parser {
    ($path:literal) => {
        ::core::include!(::core::concat!(::core::env!("OUT_DIR"), "/", $path, ".rs"));
    };
}

/// The source code of the runtime, for inlining it into generated files.
#[doc(hidden)]
pub const SOURCE: &str = include_str!("rt.rs");
