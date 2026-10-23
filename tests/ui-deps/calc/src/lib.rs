//! `calc!(x * (y + 1))`: arithmetic expressions on `i64`s, checked at compile time.

use proc_macro2::TokenStream;
use prpr::runtime::{MacroError, SourceText};
use quote::quote;
use std::range::Range;

struct Expr {
    kind: Kind,
    span: Range<usize>,
}

enum Kind {
    Num(i64),
    Var,
    Bin(Box<Expr>, &'static str, Box<Expr>),
}

fn binary(first: Expr, rest: Vec<(&'static str, Expr)>) -> Expr {
    rest.into_iter().fold(first, |l, (op, r)| Expr {
        span: Range { start: l.span.start, end: r.span.end },
        kind: Kind::Bin(Box::new(l), op, Box::new(r)),
    })
}

mod grammar {
    use super::{Expr, Kind, binary};

    prpr::grammar! {
        pub(super) expr: Expr = l:term rs:(op:("+" / "-") r:term { (op, r) })* { binary(l, rs) }

        term: Expr = l:factor rs:(op:("*" / "/") r:factor { (op, r) })* { binary(l, rs) }

        factor: Expr
            = n:number { Expr { kind: Kind::Num(n), span: @_ } }
            / var { Expr { kind: Kind::Var, span: @_ } }
            / "(" e:expr ")" { e }

        #[label = "number"]
        atom number: i64 = s:([0-9]+) &{ #s.parse::<i64>().map_err(|_| "a number that fits in `i64`") }

        #[label = "variable"]
        atom var = [a-z_] [a-z0-9_]*
    }
}

prpr::define_proc_macro! {
    /// Computes an arithmetic expression on `i64`s, such as `calc!(x * (y + 1))`.
    calc, grammar::expr, expand
}

// The Rust code that computes `e`.
fn expand(src: &SourceText, e: Expr) -> Result<TokenStream, MacroError> {
    match e.kind {
        Kind::Num(n) => Ok(quote!(#n)),
        // The variable as it was written, with its span, so that it refers to the variable of
        // the caller and errors about it point at it.
        Kind::Var => Ok(src.tokens(e.span)),
        Kind::Bin(l, op, r) => {
            if op == "/" && matches!(r.kind, Kind::Num(0)) {
                return Err(MacroError::new(r.span, "division by zero"));
            }
            let (l, r) = (expand(src, *l)?, expand(src, *r)?);
            let op: TokenStream = op.parse().unwrap();
            Ok(quote!((#l #op #r)))
        }
    }
}
