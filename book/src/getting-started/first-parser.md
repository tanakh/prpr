# Your first parser

In this chapter, we build a calculator that evaluates expressions like `1 + 2 * (3 - 4)`, one step at a time.

## A first grammar

A grammar is a list of rules.
Here is a grammar with a single rule:

```rust
prpr::grammar! {
    pub greeting = "hello" "world"
}

fn main() {
    assert_eq!(greeting("hello world"), Ok(()));
    assert!(greeting("  hello\n  world  ").is_ok());
    assert!(greeting("hello").is_err());
    assert!(greeting("helloworld").is_err());
}
```

The rule is named `greeting`, and it matches the literal `"hello"` followed by the literal `"world"`.
`grammar!` turns the rule into a function, `greeting`, that parses a whole string.
`pub` makes the function public.

A few things already happen behind the scenes:

* Whitespace between the literals, and before and after them, is skipped.
* The whole input must match; `greeting("hello")` fails because `"world"` is missing.
* Literals that look like words must be delimited: `helloworld` is one word, not `hello` followed by `world`.

The rule has no type, so its value is `()`, and the function returns `Result<(), ParseError>`.

## Values

To be useful, a rule needs a value.
Here is a rule for numbers:

```rust
prpr::grammar! {
    pub atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
}

fn main() {
    assert_eq!(number("42"), Ok(42));
    assert!(number("4 2").is_err());
}
```

Piece by piece:

* `: i64` is the type of the rule's value.
* `[0-9]` matches a character from `0` to `9`, and `+` repeats it one or more times.
* `s:` binds what follows to the name `s`.
* `{ ... }` is an *action*: Rust code that computes the value of the rule.
  In an action, `#s` is the text that `s` matched, a `&str`.
* `atom` makes the rule a *token*: whitespace is not skipped inside it.
  Without `atom`, each digit would be a token of its own, whitespace would be allowed between them, and `4 2` would match as one number whose text is `"4 2"`.

## Putting rules together

Rules refer to each other by name.
This grammar adds up numbers:

```rust
prpr::grammar! {
    pub sum: i64 = first:number rest:("+" n:number { n })* {
        first + rest.iter().sum::<i64>()
    }

    atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
}

fn main() {
    assert_eq!(sum("1 + 2 + 3"), Ok(6));
    assert_eq!(sum("42"), Ok(42));
}
```

`("+" n:number { n })` matches a `+` and a number, and its value is the number.
`*` repeats it zero or more times and collects the values into a `Vec<i64>`, which is bound to `rest`.
`number` is not `pub`, so its function is private to the module; `sum` can still use it.

## Operators and precedence

A calculator needs the four arithmetic operations, with `*` and `/` binding tighter than `+` and `-`, and parentheses.
Precedence is expressed by layering rules: an expression is a sum of terms, a term is a product of factors, and a factor is a number or an expression in parentheses.

```rust
prpr::grammar! {
    pub expr: i64 = l:term rs:(("+" / "-") term)* {
        rs.into_iter().fold(l, |a, (op, b)| if op == "+" { a + b } else { a - b })
    }

    term: i64 = l:factor rs:(("*" / "/") factor)* {
        rs.into_iter().fold(l, |a, (op, b)| if op == "*" { a * b } else { a / b })
    }

    factor: i64 = number / "(" e:expr ")" { e } / "-" f:factor { -f }

    #[label = "number"]
    atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
}

fn main() {
    assert_eq!(expr("1 + 2 * 3"), Ok(7));
    assert_eq!(expr("(1 + 2) * 3"), Ok(9));
    assert_eq!(expr("10 - 2 - 3"), Ok(5));
    assert_eq!(expr("-(4 / 2)"), Ok(-2));
}
```

`/` separates *alternatives*, which are tried in order; the first one that matches wins.

The value of `("+" / "-") term` needs a closer look.
A sequence without an action has the values of its parts as a tuple.
A literal on its own has no part in that tuple, but a choice between literals does, as the literal that matched.
So the value is `(&str, i64)`, such as `("+", 2)`, and `rs` is a `Vec<(&str, i64)>`.
`fold` then applies the operators from left to right, which makes `10 - 2 - 3` equal to `5`.

It is tempting to write `expr = expr "+" term / term` instead, but PEGs cannot express left recursion directly: the rule would call itself forever without consuming input.
prpr detects this and reports it as an error.

## Errors

When the input does not match, the function returns a `ParseError`.
Its message says what was expected at the farthest position the parser reached:

```rust
# prpr::grammar! {
#     pub expr: i64 = l:term rs:(("+" / "-") term)* {
#         rs.into_iter().fold(l, |a, (op, b)| if op == "+" { a + b } else { a - b })
#     }
#     term: i64 = l:factor rs:(("*" / "/") factor)* {
#         rs.into_iter().fold(l, |a, (op, b)| if op == "*" { a * b } else { a / b })
#     }
#     factor: i64 = number / "(" e:expr ")" { e } / "-" f:factor { -f }
#     #[label = "number"]
#     atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
# }
fn main() {
    let message = |s| expr(s).unwrap_err().to_string();
    assert_eq!(message("1 +"), "expected one of `(`, `-`, or number, found end of input");
    assert_eq!(message("1 + (2 * 3"), "expected one of `)`, `*`, `+`, `-`, or `/`, found end of input");
    assert_eq!(message("12 34"), "expected one of `*`, `+`, `-`, `/`, or end of input, found `3`");
    assert_eq!(message("1 + 2x"), "expected delimiter, found `x`");
}
```

The `#[label = "number"]` attribute on `number` makes the messages say "number" instead of the character class inside the rule, which would read "expected one of `(`, `-`, or `[0-9]`".
The last message comes from the rule that tokens must be delimited: `2x` is not a number.

`render` shows the error with the line it is on, in the style of rustc:

```rust
# prpr::grammar! {
#     pub expr: i64 = l:term rs:(("+" / "-") term)* {
#         rs.into_iter().fold(l, |a, (op, b)| if op == "+" { a + b } else { a - b })
#     }
#     term: i64 = l:factor rs:(("*" / "/") factor)* {
#         rs.into_iter().fold(l, |a, (op, b)| if op == "*" { a * b } else { a / b })
#     }
#     factor: i64 = number / "(" e:expr ")" { e } / "-" f:factor { -f }
#     #[label = "number"]
#     atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
# }
fn main() {
    let input = "1 + * 2";
    if let Err(e) = expr(input) {
        eprintln!("{}", e.render(input, "<input>"));
    }
}
```

```text
error: expected one of `(`, `-`, or number, found `*`
 --> <input>:1:5
  |
1 | 1 + * 2
  |     ^
```

## What's next

The next chapter builds a complete JSON parser and introduces the rest of the essentials: tokens with escapes, separated lists and better error messages.
