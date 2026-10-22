# Identifiers and keywords

## Identifiers

An identifier is a token, so it is an atom.
It returns its text, and a label names it in error messages:

```rust
prpr::grammar! {
    pub names: Vec<&'i str> = ident*

    #[label = "identifier"]
    atom ident: &'i str = s:([a-zA-Z_] [a-zA-Z0-9_]*) { #s }
}

fn main() {
    assert_eq!(names("foo _bar baz2"), Ok(vec!["foo", "_bar", "baz2"]));
    assert_eq!(
        names("foo 2baz").unwrap_err().to_string(),
        "expected one of identifier or end of input, found `2`"
    );
}
```

## Keywords

In syntactic rules, keywords are simply literals.
Since tokens [must be delimited](../guide/whitespace.md#delimiters), `"let"` does not match the start of `letter`:

```rust
prpr::grammar! {
    pub stmt: (&'i str, &'i str) = "let" name:ident "=" value:ident { (name, value) }

    #[label = "identifier"]
    atom ident: &'i str = s:([a-z_] [a-z0-9_]*) { #s }
}

fn main() {
    assert_eq!(stmt("let x = y"), Ok(("x", "y")));
    assert!(stmt("letx = y").is_err());
    assert_eq!(stmt("let let = let"), Ok(("let", "let")));
}
```

That is all there is to keywords that are only special where the grammar uses them, like the contextual keywords of Rust such as `union`: `let let = let` is a valid `stmt` above.

## Reserved words

To keep keywords from being identifiers, exclude them in the rule of identifiers with a negative lookahead:

```rust
prpr::grammar! {
    pub stmt: (&'i str, &'i str) = "let" name:ident "=" value:ident { (name, value) }

    #[label = "identifier"]
    atom ident: &'i str = !keyword s:([a-z_] [a-z0-9_]*) { #s }

    atom keyword = ("else" / "if" / "let") !word_char
}

fn main() {
    assert_eq!(stmt("let iffy = x"), Ok(("iffy", "x")));
    assert_eq!(
        stmt("let if = x").unwrap_err().to_string(),
        "expected identifier, found `i`"
    );
}
```

`keyword` is an atom, where nothing checks delimiters, so it checks that the keyword is a whole word with `!word_char` itself; without it, `iffy` would start with the keyword `if`.

Keep the `!word_char` after the choice, and put a keyword before the keywords it starts with, as in `"int" / "in"`.
Otherwise, with `("in" / "int") !word_char`, `in` matches the start of `int`, `!word_char` fails on `t`, and the choice is done: `int` is not a keyword.
See [Ordered choice](../guide/expressions.md#ordered-choice).

With many keywords, it is simpler to read the word and look it up:

```rust
const KEYWORDS: &[&str] = &["else", "if", "in", "int", "let"];

prpr::grammar! {
    pub stmt: (&'i str, &'i str) = "let" name:ident "=" value:ident { (name, value) }

    #[label = "identifier"]
    atom ident: &'i str = s:([a-z_] [a-z0-9_]*) &{
        if KEYWORDS.contains(&#s) { Err("identifier") } else { Ok(#s) }
    }
}

fn main() {
    assert_eq!(stmt("let iffy = x"), Ok(("iffy", "x")));
    assert_eq!(
        stmt("let int = x").unwrap_err().to_string(),
        "expected identifier, found `i`"
    );
}
```

The predicate returns an error rather than `None`.
`None` would only filter, and the error would be reported where the word could have continued, after `int`; an error is reported at the start of the word (see [Messages from predicates](../guide/errors.md#messages-from-predicates)).

## Case-insensitive keywords

In a language like SQL, where keywords are case-insensitive, match a word and compare it:

```rust
fn is_keyword(s: &str) -> bool {
    ["from", "select"].iter().any(|k| s.eq_ignore_ascii_case(k))
}

prpr::grammar! {
    pub query: Vec<&'i str> = select cols:(ident, ",")+ from ident { cols }

    #[label = "`SELECT`"]
    select = w:word &{ #w.eq_ignore_ascii_case("select") }

    #[label = "`FROM`"]
    from = w:word &{ #w.eq_ignore_ascii_case("from") }

    #[label = "identifier"]
    atom ident: &'i str = s:word &{
        if is_keyword(#s) { Err("identifier") } else { Ok(#s) }
    }

    atom word = [a-zA-Z_] [a-zA-Z0-9_]*
}

fn main() {
    assert_eq!(query("SELECT a, b FROM t"), Ok(vec!["a", "b"]));
    assert_eq!(query("select a from T"), Ok(vec!["a"]));
    assert_eq!(
        query("selec a from t").unwrap_err().to_string(),
        "expected `SELECT`, found `s`"
    );
}
```

## Unicode identifiers

Character classes list characters one by one, so identifiers with letters of every script are matched with a predicate on the character.
Redefine `word_char` along with them, so that delimiters are checked with the same letters:

```rust
prpr::grammar! {
    pub stmt: (&'i str, &'i str) = "let" name:ident "=" value:ident { (name, value) }

    #[label = "identifier"]
    atom ident: &'i str = s:(ident_start word_char*) { #s }

    atom ident_start = [a-zA-Z_] / c:. &{ c.is_alphabetic() }

    atom word_char = [a-zA-Z0-9_] / c:. &{ c.is_alphanumeric() }
}

fn main() {
    assert_eq!(stmt("let 名前 = 値"), Ok(("名前", "値")));
    assert!(stmt("let名前 = 値").is_err());
}
```

The classes for ASCII come first, since they are faster than a predicate.
For identifiers exactly as Unicode defines them, call [`unicode-ident`](https://crates.io/crates/unicode-ident) in the predicates.
