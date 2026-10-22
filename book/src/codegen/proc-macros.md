# Parsing proc-macro input

A [function-like procedural macro](https://doc.rust-lang.org/reference/procedural-macros.html#function-like-procedural-macros) can implement a small language inside Rust code, such as `sql!(SELECT ...)` or `html!(<p>...</p>)`.
Its input is a stream of Rust tokens, not text, but a prpr parser can parse it as text: `SourceText` rebuilds the text of the tokens as they were written, and maps positions in the text back to the tokens, so that errors are reported at the right place in the macro input.
This is how `grammar!` itself parses grammars.

## An example

The macro `calc!` takes an arithmetic expression on `i64` values, such as `calc!(width * (height + 1))`, checks it, and expands to Rust code that computes it.
The proc-macro crate depends on `prpr` with the feature `proc-macro2`, which provides `prpr::runtime::SourceText`, and on [proc-macro2](https://crates.io/crates/proc-macro2) and [quote](https://crates.io/crates/quote), as proc-macros usually do:

```toml
[lib]
proc-macro = true

[dependencies]
prpr = { version = "0.1", features = ["proc-macro2"] }
proc-macro2 = "1"
quote = "1"
```

The parser builds a syntax tree with the [span](../guide/values-and-actions.md#the-text-and-the-span-of-a-binding) of each expression, which the macro then turns into Rust code:

```rust,ignore
//! `calc!(x * (y + 1))`: arithmetic expressions on `i64`s, checked at compile time.

use proc_macro2::TokenStream;
use prpr::runtime::SourceText;
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

// Rust code for `e`, or the span and the message of an error.
fn expand(src: &SourceText, e: &Expr) -> Result<TokenStream, (Range<usize>, &'static str)> {
    match &e.kind {
        Kind::Num(n) => Ok(quote!(#n)),
        // The variable as it was written, with its span, so that it refers to the variable of
        // the caller and errors about it point at it.
        Kind::Var => Ok(src.tokens(e.span)),
        Kind::Bin(l, op, r) => {
            if *op == "/" && matches!(r.kind, Kind::Num(0)) {
                return Err((r.span, "division by zero"));
            }
            let (l, r) = (expand(src, l)?, expand(src, r)?);
            let op: TokenStream = op.parse().unwrap();
            Ok(quote!((#l #op #r)))
        }
    }
}

#[proc_macro]
pub fn calc(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let src = SourceText::new(input.into());
    let expr = match grammar::expr(src.text()) {
        Ok(expr) => expr,
        Err(e) => return src.parse_error(&e).into(),
    };
    match expand(&src, &expr) {
        Ok(code) => code.into(),
        Err((span, message)) => src.compile_error(span, message).into(),
    }
}
```

A crate that uses the macro:

```rust,ignore
use calc::calc;

fn main() {
    let (width, height) = (3, 4);
    assert_eq!(calc!(width * (height + 1)), 15);
    println!("{}", calc!(100 / width - 1)); // 32
}
```

The three kinds of errors the macro can meet are reported at the tokens they are about:

```rust,ignore
let a = calc!(width + * height);
let b = calc!(width / 0);
let c = calc!(width * hieght);
```

<pre class="terminal"><code class="nohighlight"><span class="ansi-91 ansi-bold">error</span><span class="ansi-bold">: expected one of `(`, number, or variable, found `*`</span>
 <span class="ansi-94 ansi-bold">--&gt; </span>app/src/main.rs:5:27
  <span class="ansi-94 ansi-bold">|</span>
<span class="ansi-94 ansi-bold">5</span> <span class="ansi-94 ansi-bold">|</span>     let a = calc!(width + * height);
  <span class="ansi-94 ansi-bold">|</span>                           <span class="ansi-91 ansi-bold">^</span>

<span class="ansi-91 ansi-bold">error</span><span class="ansi-bold">: division by zero</span>
 <span class="ansi-94 ansi-bold">--&gt; </span>app/src/main.rs:6:27
  <span class="ansi-94 ansi-bold">|</span>
<span class="ansi-94 ansi-bold">6</span> <span class="ansi-94 ansi-bold">|</span>     let b = calc!(width / 0);
  <span class="ansi-94 ansi-bold">|</span>                           <span class="ansi-91 ansi-bold">^</span>

<span class="ansi-91 ansi-bold">error[E0425]</span><span class="ansi-bold">: cannot find value `hieght` in this scope</span>
 <span class="ansi-94 ansi-bold">--&gt; </span>app/src/main.rs:7:27
  <span class="ansi-94 ansi-bold">|</span>
<span class="ansi-94 ansi-bold">7</span> <span class="ansi-94 ansi-bold">|</span>     let c = calc!(width * hieght);
  <span class="ansi-94 ansi-bold">|</span>                           <span class="ansi-91 ansi-bold">^^^^^^</span>
  <span class="ansi-94 ansi-bold">|</span>
<span class="ansi-96 ansi-bold">help</span>: a local variable with a similar name exists
  <span class="ansi-94 ansi-bold">|</span>
<span class="ansi-94 ansi-bold">7</span> <span class="ansi-91">- </span>    let c = calc!(width * <span class="ansi-91">hieght</span>);
<span class="ansi-94 ansi-bold">7</span> <span class="ansi-92">+ </span>    let c = calc!(width * <span class="ansi-92">height</span>);
  <span class="ansi-94 ansi-bold">|</span></code></pre>

* The parse error is reported by `parse_error`, at the token where the parser failed.
* The division by zero is found by the macro, which reports it with `compile_error` at the span of the divisor.
* The misspelled variable is found by the compiler, since the variable is passed on with its original token.

## `SourceText`

`SourceText::new` takes the input of the macro, as a `proc_macro2::TokenStream`:

| method | returns |
|---|---|
| `text()` | The text of the tokens, to parse. |
| `parse_error(&e)` | A `compile_error!` invocation that reports a `ParseError` at the tokens it is about. |
| `compile_error(range, message)` | A `compile_error!` invocation that reports `message` at the tokens in a range of the text. |
| `tokens(range)` | The original tokens in a range of the text, with their spans. |
| `span(offset)` | The span of the token at an offset of the text. |
| `span_range(range)` | The spans of the first and the last token in a range of the text. |

Ranges and offsets are in bytes, as the spans of bindings (`@x`) and the offsets of errors are.
An error reported over several tokens is shown from the first to the last of them.

`tokens` is the way to pass parts of the input on to the generated code.
Tokens keep their spans, so that names in them refer to what is in scope where the macro is invoked, and errors about them are reported where they were written, as for `hieght` above.
A language that embeds Rust expressions can likewise parse them as text, such as a balanced group `{ ... }`, and pass on their tokens.

## What the text looks like

The text is rebuilt from the positions of the tokens, so it has the line breaks and the spaces of the source, except that comments become spaces.
Where the positions of tokens are not available or not consistent, as for tokens produced by `macro_rules!` macros, tokens are separated by single spaces, and punctuation written together, such as `<=`, is kept together.

The input of a function-like macro must consist of Rust tokens, which the compiler checks before the macro runs: brackets must be balanced, and string and character literals must be complete.
A language whose syntax goes beyond Rust tokens, such as one with unbalanced quotes, cannot be parsed this way.

## Grammars in proc-macro crates

A proc-macro crate can only export macros.
Its rules and the types of their values must therefore not be `pub`; as in the example, put the grammar in a module and make the rules that the macro calls `pub(super)` or `pub(crate)`.
