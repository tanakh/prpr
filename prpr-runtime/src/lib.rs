#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

mod rt;
pub use rt::*;

const _: () = assert!(
    VERSION.0 == parse_u32(env!("CARGO_PKG_VERSION_MAJOR"))
        && VERSION.1 == parse_u32(env!("CARGO_PKG_VERSION_MINOR")),
    "`VERSION` in rt.rs must be the version of the package"
);

// A number of decimal digits.
const fn parse_u32(digits: &str) -> u32 {
    let digits = digits.as_bytes();
    let mut n = 0;
    let mut i = 0;
    while i < digits.len() {
        n = n * 10 + (digits[i] - b'0') as u32;
        i += 1;
    }
    n
}

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
