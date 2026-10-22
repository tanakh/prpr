# Comments

Comments are skipped with whitespace, by the built-in rule `spaces`, which is `(whitespace / comment)*`.
A grammar defines the rule `comment`, an atom that matches one comment, and comments are then allowed wherever whitespace is (see [What is skipped](../guide/whitespace.md#what-is-skipped)).

## Line and block comments

```rust
prpr::grammar! {
    pub words: Vec<&'i str> = word*

    atom word: &'i str = s:([a-z]+) { #s }

    atom comment = "//" (!"\n" .)* / "/*" (!"*/" .)* "*/"
}

fn main() {
    assert_eq!(words("a // b\n c /* d */ e"), Ok(vec!["a", "c", "e"]));
    assert_eq!(words("a/* b */c"), Ok(vec!["a", "c"]));
}
```

A comment separates tokens, as whitespace does, so `a/* b */c` is two words.

An unterminated block comment is not a comment, so the error is reported where it starts:

```rust
# prpr::grammar! {
#     pub words: Vec<&'i str> = word*
#
#     atom word: &'i str = s:([a-z]+) { #s }
#
#     atom comment = "//" (!"\n" .)* / "/*" (!"*/" .)* "*/"
# }
fn main() {
    let e = words("a /* b").unwrap_err();
    assert_eq!(e.offset(), 2);
    assert_eq!(e.to_string(), "expected one of `[a-z]` or end of input, found `/`");
}
```

## Nested comments

A block comment that may contain block comments refers to itself:

```rust
prpr::grammar! {
    pub words: Vec<&'i str> = word*

    atom word: &'i str = s:([a-z]+) { #s }

    atom comment = "//" (!"\n" .)* / block_comment

    atom block_comment = "/*" (block_comment / !"*/" .)* "*/"
}

fn main() {
    assert_eq!(words("a /* b /* c */ d */ e"), Ok(vec!["a", "e"]));
}
```

## Comments in line-based languages

In a language where line breaks matter, the line break is not part of the comment, nor of `whitespace`:

```rust
prpr::grammar! {
    pub lines: Vec<Vec<&'i str>> = (line, "\n")*

    line: Vec<&'i str> = word*

    atom word: &'i str = s:([a-z]+) { #s }

    atom whitespace = [' ' '\t']

    atom comment = "#" (!"\n" .)*
}

fn main() {
    assert_eq!(
        lines("a b # c\nd # e\n# f\ng"),
        Ok(vec![vec!["a", "b"], vec!["d"], vec![], vec!["g"]])
    );
}
```

## Comments that matter

Comments that the grammar uses, such as documentation comments, must not be skipped.
Leave them out of `comment`, and match them as tokens where they are allowed:

```rust
prpr::grammar! {
    pub items: Vec<(Vec<&'i str>, &'i str)> = item*

    item: (Vec<&'i str>, &'i str) = docs:doc* "fn" name:ident { (docs, name) }

    // `///` starts a documentation comment, but `////` is an ordinary comment.
    atom doc: &'i str = "///" !"/" s:(!"\n" .)* { #s.trim() }

    atom comment = "//" !("/" !"/") (!"\n" .)*

    atom ident: &'i str = s:([a-z]+) { #s }
}

fn main() {
    let input = "
        /// Does this.
        /// And that.
        fn f
        // An ordinary comment.
        //// Another one.
        fn g
    ";
    assert_eq!(items(input), Ok(vec![(vec!["Does this.", "And that."], "f"), (vec![], "g")]));
}
```

A documentation comment that is not before an item is then a syntax error, which may or may not be what the language wants; Rust, for one, reports it as an error.
