# Whitespace, tokens and delimiters

prpr has no separate lexer.
Instead, rules come in two kinds:

* **Syntactic rules**, the default, describe how tokens are put together.
  Whitespace is skipped before each token.
* **Atoms**, rules marked with `atom`, describe tokens.
  Nothing is skipped inside them.

## Tokens

In a syntactic rule, these are tokens:

* literals, such as `"if"` and `'('`;
* character classes and `.`;
* references to atoms;
* atomic groups, `<e>`.

Before each token, the rule `spaces` is applied, which skips whitespace.
References to other syntactic rules are not tokens: those rules skip whitespace before their own tokens.

```rust
prpr::grammar! {
    pub call: (&'i str, Vec<&'i str>) = f:ident "(" args:(ident, ",")* ")" { (f, args) }

    atom ident: &'i str = s:([a-z_] [a-z0-9_]*) { #s }
}

fn main() {
    assert_eq!(call(" f ( x , y ) "), Ok(("f", vec!["x", "y"])));
    assert_eq!(call("f(x,y)"), Ok(("f", vec!["x", "y"])));
    assert!(call("f(x y)").is_err());
}
```

An entry function of a syntactic rule, such as `call`, also skips whitespace at the end of the input.
Spans and the text of bindings (`@x`, `#x`) never include the whitespace before them.

## What is skipped

`spaces` is defined as `(whitespace / comment)*`, with these defaults:

```rust,ignore
atom whitespace = [' ' '\t' '\r' '\n']
atom comment = []    // never matches: there are no comments
```

A grammar can redefine `whitespace`, `comment` or `spaces` itself.
They must be atoms, and their failures are never reported in error messages.

Most languages only need to define `comment`:

```rust
prpr::grammar! {
    pub words: Vec<&'i str> = word*

    atom word: &'i str = s:([a-z]+) { #s }

    atom comment = "//" (!"\n" .)* / "/*" (!"*/" .)* "*/"
}

fn main() {
    assert_eq!(words("a // b\n c /* d */ e"), Ok(vec!["a", "c", "e"]));
}
```

For a language where line breaks matter, leave them out of `whitespace` and use them explicitly:

```rust
prpr::grammar! {
    pub lines: Vec<Vec<&'i str>> = (line, "\n")*

    line: Vec<&'i str> = word*

    atom word: &'i str = s:([a-z]+) { #s }

    atom whitespace = [' ' '\t']
}

fn main() {
    assert_eq!(lines("a b\n c"), Ok(vec![vec!["a", "b"], vec!["c"]]));
}
```

## Atoms that contain syntax

Whether whitespace is skipped depends only on the rule that is being matched.
So an atom can refer to a syntactic rule, which then skips whitespace before its own tokens, as in string interpolation:

```rust
prpr::grammar! {
    pub atom string: Vec<&'i str> = '"' parts:part* '"' { parts }

    atom part: &'i str = "${" e:expr spaces "}" { e } / s:([a-z ' ']+) { #s }

    expr: &'i str = s:ident { s }

    atom ident: &'i str = s:([a-z]+) { #s }
}

fn main() {
    assert_eq!(string(r#""a ${ x } b""#), Ok(vec!["a ", "x", " b"]));
}
```

`expr` skips the whitespace before `x`, but the whitespace after it is inside the atom `part`, which has to skip it itself with `spaces`.

## Delimiters

Without a lexer, nothing says where a token ends, so prpr checks that tokens are delimited.
After a literal, a reference to an atom or an atomic group in a syntactic rule, if the token ends with a *word character* and the next character is one too, the token does not match.
That is what keeps `"if"` from matching the start of `ifx`, and a number from matching `123` in `123abc`.

The check looks at both sides of the end of the token.
A token that ends with a symbol, such as `(` or `+`, may be followed by anything, so `f(x)` and `a+b` need no whitespace.
Requiring a non-word character after *every* token would reject them.

```rust
prpr::grammar! {
    pub statement = "if" ident / ident
    pub numbers = number+

    atom ident = [a-z]+
    atom number = [0-9]+
}

fn main() {
    assert!(statement("if x").is_ok());
    assert!(statement("ifx").is_ok()); // the identifier `ifx`, not `if` followed by `x`
    assert!(numbers("12 34").is_ok());
    assert_eq!(
        numbers("123abc").unwrap_err().to_string(),
        "expected delimiter, found `a`"
    );
}
```

Word characters are defined by the rule `word_char`, `[a-zA-Z0-9_]` by default, which a grammar can redefine:

```rust
prpr::grammar! {
    pub kw = "let" name

    atom name = [a-z]+

    // Identifiers may contain `-`, so `let-x` is not `let` followed by `-x`.
    atom word_char = [a-z0-9_-]
}

fn main() {
    assert!(kw("let x").is_ok());
    assert!(kw("let-x").is_err());
}
```

Single characters, from character classes and `.`, are not words and are never checked.
Neither is anything inside an atom.

For an atom, the check can be replaced with `#[boundary(e)]`, which is written as an expression rather than with word characters: the token matches only if `e` matches after it (without consuming anything).
`#[no_boundary]` turns the check off.

```rust
prpr::grammar! {
    pub numbers: Vec<&'i str> = number*

    // A number must not be followed by a letter, a digit or a `.`.
    #[boundary(![a-z0-9.])]
    atom number: &'i str = s:([0-9]+) { #s }
}

fn main() {
    assert_eq!(numbers("1 2 3"), Ok(vec!["1", "2", "3"]));
    assert_eq!(
        numbers("1 2.x").unwrap_err().to_string(),
        "expected delimiter, found `.`"
    );
}
```

A token that is not delimited is reported as "expected delimiter".
