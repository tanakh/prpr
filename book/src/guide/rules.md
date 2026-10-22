# Rules

A grammar is a sequence of rules and Rust items.
A rule has this form:

```text
[attributes] [pub] [atom] name [: Type] = expression
```

```rust
mod words {
    prpr::grammar! {
        /// A comma-separated list of words.
        pub list: Vec<&'i str> = (word, ",")*

        #[label = "word"]
        pub(crate) atom word: &'i str = s:([a-z]+) { #s }

        comma = ","
    }
}

fn main() {
    assert_eq!(words::list("foo, bar"), Ok(vec!["foo", "bar"]));
    assert_eq!(words::word("baz"), Ok("baz"));
}
```

A rule ends where the next rule or Rust item begins, so rules need no terminator.

## Names

Rule names are Rust identifiers, and keywords such as `type` or `match` cannot be used.
`atom` cannot be used either.

## Types

The type after the name is the type of the rule's value.
It is a Rust type made of paths with generic arguments, references, tuples, arrays, `dyn` and `impl` bounds and function pointers, such as `i64`, `Vec<(String, Expr)>`, `Box<dyn Fn(i32) -> i32>` or `[u8; 4]`.
Qualified paths such as `<T as Trait>::Output` are not supported; define a type alias and use that.

The lifetime `'i` is the lifetime of the input, so a rule can return parts of the input without copying them, like `word` above.

A rule without a type, like `comma`, only recognizes input; its value is `()`.
A rule of type `()` is the same as a rule without a type.

Note that the value of a rule without a type is `()`, not the text it matched.
To get the text, either take it where the rule is used, with `#x` on a binding, or give the rule the type `&'i str` and return the text from the rule itself:

```rust
prpr::grammar! {
    // `name` has no type: its value is `()`, and `#n` is the text it matched.
    pub greeting: &'i str = "hello" n:name { #n }

    atom name = [a-z]+

    // `word` returns its text as its value.
    pub words: Vec<&'i str> = word*

    atom word: &'i str = s:([a-z]+) { #s }
}

fn main() {
    assert_eq!(greeting("hello world"), Ok("world"));
    assert_eq!(words("a bc"), Ok(vec!["a", "bc"]));
}
```

The first way keeps `name` cheap where only its presence matters; the second is convenient when the text is used in many places.

## Generated functions

For each rule, `grammar!` generates a function with the rule's name and visibility that parses a whole input:

```rust,ignore
pub fn list<'i>(input: &'i str) -> Result<Vec<&'i str>, prpr::ParseError>;
```

The function skips whitespace at the start and at the end (unless the rule is an `atom`) and fails unless the rule matches all of the input.
Doc comments on the rule become the documentation of the function.
Rules without `pub` get private functions, which are still useful in tests.

Each rule also gets a function in the module `rules` that other grammars and Rust code can call; see [Composing grammars](composing.md).

The generated items are put in the scope where `grammar!` is invoked: a function for each rule, the module `rules`, the struct `Memo` and a few helpers whose names start with `__`.
To keep them apart from other code, put each grammar in a module of its own.
`grammar!` must be invoked where items can be defined at the module level, not inside a function.

## Atoms

`atom` makes a rule a token: whitespace is not skipped inside it, and where it is used in another rule, it is treated as one token.
See [Whitespace, tokens and delimiters](whitespace.md).

## Attributes

| attribute | meaning |
|---|---|
| `/// ...` | Documentation of the generated function. |
| `#[label = "..."]` | What to call the rule in error messages. See [Error messages](errors.md). |
| `#[memo]` | Remember the results of the rule. See [Memoization and performance](memoization.md). |
| `#[boundary(e)]` | On an atom: `e` must match after the token. See [Delimiters](whitespace.md#delimiters). |
| `#[no_boundary]` | On an atom: no delimiter is required after the token. |

Attributes that apply to the whole grammar are written at its start with `#!`:

| attribute | meaning |
|---|---|
| `#![input = str]` | The input is text, `&str`. This is the default. |
| `#![input = bytes]` | The input is binary, `&[u8]`. See [Binary input](binary-input.md). |

## Rust items

`use` declarations, functions, structs, enums, `impl` blocks and other Rust items can be written among the rules.
They are copied to the output as they are, so the code in actions can use them:

```rust
mod config {
    prpr::grammar! {
        use std::collections::HashMap;

        fn insert_all(pairs: Vec<(&str, i64)>) -> HashMap<String, i64> {
            pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
        }

        pub config: HashMap<String, i64> = ps:(pair, ";")* { insert_all(ps) }

        pair: (&'i str, i64) = k:key "=" v:number { (k, v) }

        atom key: &'i str = s:([a-z]+) { #s }

        atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
    }
}

fn main() {
    let c = config::config("a = 1; b = 2").unwrap();
    assert_eq!(c["b"], 2);
}
```

## Built-in rules

These rules are defined unless the grammar defines them itself:

| rule | definition | purpose |
|---|---|---|
| `whitespace` | `[' ' '\t' '\r' '\n']` | One character of whitespace. |
| `comment` | (matches nothing) | One comment. |
| `spaces` | `(whitespace / comment)*` | What is skipped before tokens. |
| `word_char` | `[a-zA-Z0-9_]` | A character of a word. A token that ends with one must not be followed by another one. |
| `eof` | `!.` | The end of the input. |

`spaces` and `word_char` are how prpr splits the input into tokens without a lexer.
Rules that are not atoms use them implicitly: `spaces` is applied before each token, and after a token that ends with a word character, the next character must not be a word character.
So `if x` is the keyword `if` followed by `x`, while `ifx` is one word, and `f(x)` and `a+b` need no whitespace at all.
Atoms use neither: inside an atom, every character is matched explicitly.
[Whitespace, tokens and delimiters](whitespace.md) explains this in detail, and how to redefine these rules.

## Recursion

Rules may refer to each other recursively, as long as a rule consumes input before it is reached again.
A rule that can reach itself without consuming input, such as `expr = expr "+" term / term`, is *left-recursive*, and prpr reports an error:

```text
error: left recursion: `expr` -> `expr`
       = note: a rule must consume input before it is invoked again; left recursion is not supported
```

Use repetition instead, as in [the calculator](../getting-started/first-parser.md#operators-and-precedence).
