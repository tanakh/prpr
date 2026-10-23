#![doc = include_str!("../README.md")]

use core::range::Range;
use proc_macro::TokenStream;
use proc_macro2::{Delimiter, Literal, Span, TokenTree};
use prpr_codegen::{Diagnostic, Options, Source, TextSource};
use prpr_runtime::SourceText;
use quote::{quote, quote_spanned};
use std::path::{Path, PathBuf};

fn options() -> Options {
    Options {
        runtime: quote!(::prpr::runtime),
        check_version: false,
    }
}

/// Generates a parser from a grammar: for each rule, a function that parses a whole input with
/// it, the type `Rules` with an associated function for each rule, and the struct `Memo`.
///
/// [The prpr Book](https://tanakh.github.io/prpr/) explains how to write grammars.
#[proc_macro]
pub fn grammar(input: TokenStream) -> TokenStream {
    let src = SourceText::new(input.into());
    match prpr_codegen::compile(&src, &options()) {
        Ok(tokens) => tokens.into(),
        Err(diags) => diags
            .iter()
            .filter(|d| d.is_error())
            .map(|d| {
                let text = d.to_text(|span| format!("line {}", src.span(span.start).start().line));
                src.compile_error(d.span, &text)
            })
            .collect::<proc_macro2::TokenStream>()
            .into(),
    }
}

/// Generates a parser from a grammar file, as `grammar!` does from a grammar in Rust code.
/// The path is relative to the file that invokes the macro, as for `include_str!`; where the
/// compiler does not tell which file that is, relative to the directory of `Cargo.toml`.
#[proc_macro]
pub fn include_grammar(input: TokenStream) -> TokenStream {
    let Some((lit, path)) = path_literal(input.into()) else {
        return error(
            Span::call_site(),
            "expected the path of a grammar file, as a string literal",
        );
    };
    let (path, shown) = resolve(&path);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) => return error(lit.span(), &format!("cannot read {}: {e}", shown.display())),
    };
    let src = Included {
        text: TextSource(&text),
        path: lit.span(),
    };
    match prpr_codegen::compile(&src, &options()) {
        Ok(tokens) => {
            // Makes the compiler build the crate again when the file changes.
            let path = path.canonicalize().unwrap_or(path);
            let path = path.to_string_lossy();
            quote! {
                #tokens
                const _: &str = ::core::include_str!(#path);
            }
            .into()
        }
        Err(diags) => diags
            .iter()
            .filter(|d| d.is_error())
            .map(|d| {
                let message = rendered(d, &text, &shown);
                quote_spanned!(lit.span()=> ::core::compile_error!(#message);)
            })
            .collect::<proc_macro2::TokenStream>()
            .into(),
    }
}

// The string literal that is the only token of `input`, and its value.
fn path_literal(input: proc_macro2::TokenStream) -> Option<(Literal, String)> {
    let mut tokens = input.into_iter();
    let lit = match (tokens.next()?, tokens.next()) {
        (TokenTree::Literal(lit), None) => lit,
        // A literal passed through a `macro_rules!` macro.
        (TokenTree::Group(g), None) if g.delimiter() == Delimiter::None => {
            return path_literal(g.stream());
        }
        _ => return None,
    };
    let value = string_value(&lit.to_string())?;
    Some((lit, value))
}

// The value of a string literal written as `text`, if it is one.
fn string_value(text: &str) -> Option<String> {
    if let Some(raw) = text.strip_prefix('r') {
        let hashes = raw.len() - raw.trim_start_matches('#').len();
        let body = raw.get(hashes..raw.len() - hashes)?;
        return Some(body.strip_prefix('"')?.strip_suffix('"')?.to_string());
    }
    let body = text.strip_prefix('"')?.strip_suffix('"')?;
    let mut value = String::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            value.push(c);
            continue;
        }
        value.push(match chars.next()? {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '0' => '\0',
            c @ ('\\' | '\'' | '"') => c,
            'x' => {
                let code: String = chars.by_ref().take(2).collect();
                char::from(
                    u8::from_str_radix(&code, 16)
                        .ok()
                        .filter(|b| b.is_ascii())?,
                )
            }
            'u' => {
                let code: String = chars.by_ref().skip(1).take_while(|&c| c != '}').collect();
                char::from_u32(u32::from_str_radix(&code, 16).ok()?)?
            }
            // A line continuation skips the line break and the whitespace after it.
            '\n' => {
                while chars.clone().next().is_some_and(char::is_whitespace) {
                    chars.next();
                }
                continue;
            }
            _ => return None,
        });
    }
    Some(value)
}

// The file at `path` relative to the file that invokes the macro, and how to show it in messages.
fn resolve(path: &str) -> (PathBuf, PathBuf) {
    let base = proc_macro::Span::call_site()
        .local_file()
        .and_then(|file| file.parent().map(Path::to_path_buf))
        .or_else(|| std::env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from))
        .unwrap_or_default();
    // Paths relative to the directory where the compiler runs, as in its own messages.
    let shown = base.join(path);
    let path = match std::env::current_dir() {
        Ok(dir) if shown.is_relative() => dir.join(&shown),
        _ => shown.clone(),
    };
    (path, shown)
}

// A diagnostic about the grammar file `shown`, with the line it is about, without the leading
// "error: " that the compiler adds itself.
fn rendered(d: &Diagnostic, text: &str, shown: &Path) -> String {
    let rendered = d.render(text, &shown.display().to_string());
    match rendered.strip_prefix("error: ") {
        Some(rest) => rest.to_string(),
        None => rendered,
    }
}

fn error(span: Span, message: &str) -> TokenStream {
    quote_spanned!(span=> ::core::compile_error!(#message);).into()
}

// A grammar file. The names generated for its rules are located at the path, the only place in
// the source that a macro can point at, and a place written by the user, so that the compiler
// lints them, e.g. reports unused rules with `#![lint]`.
struct Included<'a> {
    text: TextSource<'a>,
    path: Span,
}

impl Source for Included<'_> {
    fn text(&self) -> &str {
        self.text.text()
    }

    fn tokens(&self, span: Range<usize>) -> Result<proc_macro2::TokenStream, Diagnostic> {
        self.text.tokens(span)
    }

    fn token_span(&self, _span: Range<usize>) -> Span {
        self.path
    }

    fn locate(&self, span: Span) -> Option<Range<usize>> {
        self.text.locate(span)
    }
}
