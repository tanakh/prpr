# Expressions

The body of a rule is an expression.
prpr's expressions are those of [Parsing Expression Grammars](https://en.wikipedia.org/wiki/Parsing_expression_grammar) (PEGs), as defined by Bryan Ford, with a few additions: bindings, actions and semantic predicates, which compute values, and two shorthands, separated lists and atomic groups.
This table lists all kinds of expressions, from the loosest to the tightest binding, and where each comes from:

| expression | matches | from |
|---|---|---|
| `e1 / e2` | `e1`, or else `e2` | PEG |
| `e1 e2` | `e1` followed by `e2` | PEG |
| `e1 e2 { code }`, `e1 e2 &{ code }` | a sequence with an action or a semantic predicate (see [Values and actions](values-and-actions.md)) | prpr |
| `name:e` | `e`, binding its value to `name` | prpr |
| `&e`, `!e` | nothing, if `e` matches here (`&`) or does not (`!`) | PEG |
| `e*`, `e+`, `e?` | `e` zero or more times, one or more times, or optionally | PEG |
| `(e)` | `e` | PEG |
| `(e, sep)*`, `(e, sep)+` | `e` repeated with `sep` between | prpr, shorthand for `(e (sep e)*)?` and `e (sep e)*` |
| `<e>` | `e` as one token | prpr: part of a rule matched as an atom |
| `"abc"`, `'c'` | a literal | PEG, in Rust syntax |
| `[a-z_]`, `[^ '"']` | a character in a class, or not in it | PEG, with Rust character literals |
| `.` | any character | PEG |
| `rule`, `path::to::rule` | a rule | PEG; paths to other grammars are prpr's |

The additions do not change which inputs a grammar can describe, except that semantic predicates can reject a match with Rust code, as in many PEG tools.
Besides, in rules that are not atoms, prpr skips whitespace and checks delimiters implicitly; see [Whitespace, tokens and delimiters](whitespace.md).

PEGs differ from other kinds of grammars in two ways that matter in practice: choices are ordered, and repetitions are greedy.
Both are explained below.

## Literals

String literals use the syntax of Rust string literals, including escapes such as `"\n"`, `"\u{3042}"` and `"\""`, and raw strings such as `r"\d+"` and `r#"a "quoted" word"#` (with up to three `#`s).
A character literal like `'"'` is a literal of one character.
Literals are case-sensitive.

With [binary input](binary-input.md), byte strings like `b"\x89PNG"` and byte literals like `b'\0'` are literals too.

## Character classes

A class matches one character that is in it, and `^` right after the `[` negates it.
Classes are part of PEGs; what is particular to prpr is how their items are written.
Inside the brackets, each item is either a character that stands for itself, or a Rust character literal; `a-b` is a range of either kind, and whitespace between items is ignored.
A class contains only characters: string literals such as `"a"` cannot be used in it.

```rust,ignore
[a-zA-Z_]             // letters and `_`
[0-9 a-f A-F]         // hexadecimal digits
[^ '"' '\\']          // anything but a quote or a backslash
[' ' '\t']            // space or tab
['\u{3040}'-'\u{309f}'] // hiragana
[+-]                  // `+` or `-`: a `-` at either end stands for itself
```

Grammars are written with Rust tokens (they can appear inside `grammar!`), so some characters must be written as character literals inside a class:

* whitespace, `[`, `]`, `'`, `"` and `\`;
* `/` when it would form `//` or `/*`, which start comments.

For the same reason, a digit directly followed by `e` or `E` is read as an incomplete number, so `[0-9e]` is an error; write `[0-9 e]` or `[e0-9]` instead.

## Any character

`.` matches any one character, and fails only at the end of the input.
`!.` therefore matches only at the end of the input; the built-in rule `eof` is defined that way.

## Rules

A rule name refers to a rule of the same grammar.
A path such as `lexer::ident` refers to a rule of another grammar; see [Composing grammars](composing.md).

## Sequences

`e1 e2 e3` matches its elements one after another.
In rules that are not atoms, whitespace may appear before each token; see [Whitespace, tokens and delimiters](whitespace.md).

## Ordered choice

`e1 / e2` tries `e1` first.
If `e1` matches, the choice is done, even if what follows fails later; `e2` is only tried when `e1` does not match.
So the order of alternatives matters when one could match a prefix of what another matches:

```rust
prpr::grammar! {
    pub wrong = "<" / "<="
    pub right = "<=" / "<"
}

fn main() {
    assert!(wrong("<=").is_err()); // `<` matches, and `=` is left over
    assert!(right("<=").is_ok());
    assert!(right("<").is_ok());
}
```

Put longer alternatives first.
Literals that end with a letter or digit are less of a problem: `"if" / "ifdef"` works, because `if` does not match the start of `ifdef` (see [Delimiters](whitespace.md#delimiters)).

## Repetition

`e*` matches `e` as many times as possible, `e+` at least once, and `e?` once or not at all.
Repetitions are greedy and never give back what they matched, so `"-"* "-"` matches nothing: the repetition takes every `-`, and none is left for the last one.

```rust
prpr::grammar! {
    pub dashes = "-"* "-"
    pub arrow = ("-" !">")* "->"
}

fn main() {
    assert!(dashes("---").is_err());
    assert!(arrow("--->").is_ok());
}
```

As `arrow` shows, a lookahead in the repeated expression decides where to stop.

Repeating an expression that can match the empty input, such as `("a"?)*`, would never end, so prpr rejects it.

## Separated lists

`(e, sep)*` matches zero or more `e`s with `sep` between them, and `(e, sep)+` one or more.
They are prpr's shorthands: they match the same as `(e (sep e)*)?` and `e (sep e)*`, but their values are simply vectors of the values of the `e`s.
A separator after the last element is not part of the list, so a trailing comma in `[1, 2,]` is an error unless the grammar allows it:

```rust
prpr::grammar! {
    pub strict: Vec<char> = "[" xs:([a-z], ",")* "]" { xs }
    pub lenient: Vec<char> = "[" xs:([a-z], ",")* ","? "]" { xs }
}

fn main() {
    assert!(strict("[a, b,]").is_err());
    assert_eq!(lenient("[a, b,]"), Ok(vec!['a', 'b']));
}
```

## Lookahead

`&e` matches if `e` matches at the current position, and `!e` if it does not.
Neither consumes any input.
Lookaheads are commonly used to exclude something, as in `!keyword identifier`, or to stop a repetition, as in `"/*" (!"*/" .)* "*/"`.

## Atomic groups

`<e>` embeds an atom in a rule that is not one: `e` is matched without skipping whitespace inside it, and the whole is treated as one token, with whitespace skipped before it and a delimiter checked after it.
This makes part of a rule a token on the spot, without defining and naming an atom for it:

```rust
prpr::grammar! {
    pub version: &'i str = "v" v:<[0-9]+ "." [0-9]+> { #v }
}

fn main() {
    assert_eq!(version("v 1.2"), Ok("1.2"));
    assert!(version("v 1 .2").is_err());
}
```

Inside an atom, where nothing is skipped anyway, `<e>` is the same as `e`.

## Grouping

Parentheses group an expression, e.g. `("+" / "-") term`.
