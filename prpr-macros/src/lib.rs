//! The `grammar!` macro of prpr. Use it through the `prpr` crate.

use proc_macro::TokenStream;
use prpr_runtime::SourceText;

/// Generates a parser from a grammar.
#[proc_macro]
pub fn grammar(input: TokenStream) -> TokenStream {
    let src = SourceText::new(input.into());
    let opts = prpr_codegen::Options {
        runtime: quote::quote!(::prpr::runtime),
        check_version: false,
    };
    match prpr_codegen::compile(&src, &opts) {
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
