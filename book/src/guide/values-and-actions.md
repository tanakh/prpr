# Values and actions

Every expression has a value, and rules with a type return the value of their body.
Actions compute values with Rust code, and semantic predicates decide with Rust code whether something matches.

## The values of expressions

| expression | value | silent |
|---|---|---|
| a literal, `"if"` | `&'static str` | yes |
| a choice between literals, `"+" / "-"` | the literal that matched, `&'static str` | no |
| a class, `[a-z]`, or `.` | `char` | no |
| a rule with type `T` | `T` | no |
| a rule without a type | `()` | yes |
| `e?` | `Option<T>` | if `e` is |
| `e*`, `e+`, `(e, sep)*`, `(e, sep)+` | `Vec<T>` | if `e` is |
| `&e`, `!e` | `()` | yes |
| `(e)`, `<e>`, `name:e` | the value of `e` | if `e` is |
| a sequence without an action | the tuple of the values of its elements that are not silent | if all elements are |
| a sequence with an action | the value of the action | no |
| `e1 / e2` | the value of the alternative that matched | if all alternatives are |

With [binary input](binary-input.md), literals are `&'static [u8]` (unless written as strings) and classes and `.` are `u8`.

A *silent* value is left out of the tuple of a sequence.
That is what makes the value of `"(" expr ")"` simply the value of `expr`, and the value of `name ":" ty` a pair:

```rust
prpr::grammar! {
    pub pair: (char, char) = "(" [a-z] "," [a-z] ")"
    pub signed: (&'static str, char) = ("+" / "-") [0-9]
    pub optional: Option<char> = "(" [a-z]? ")"
    pub many: Vec<(char, char)> = ([a-z] [0-9])*
}

fn main() {
    assert_eq!(pair("(a, b)"), Ok(('a', 'b')));
    assert_eq!(signed("- 7"), Ok(("-", '7')));
    assert_eq!(optional("()"), Ok(None));
    assert_eq!(many("a1 b2"), Ok(vec![('a', '1'), ('b', '2')]));
}
```

The alternatives of a choice must have values of the same type, unless the choice is silent.

## Actions

An action is Rust code in braces at the end of a sequence.
It computes the value of the sequence, and it can use the values of the elements that are bound to names with `name:`:

```rust
prpr::grammar! {
    pub point: (i64, i64) = "(" x:int "," y:int ")" { (x, y) }

    atom int: i64 = s:("-"? [0-9]+) { #s.parse().unwrap() }
}

fn main() {
    assert_eq!(point("(1, -2)"), Ok((1, -2)));
}
```

* A binding is visible in the action of the sequence it belongs to.
  A sequence in parentheses can have an action of its own, like `("+" n:number { n })` in [the calculator](../getting-started/first-parser.md#putting-rules-together).
* The value of an action whose sequence is the whole body of a rule has the type of the rule; other actions have their types inferred.
* An action is the body of a closure, so `return` and `?` in it leave the action, not the parser.
* An action can also stand alone, as in `"null" { Value::Null }`, or even without anything before it, as in `x / { 0 }`, where it matches the empty input.

## The text and the span of a binding

In an action:

| | is |
|---|---|
| `#x` | the input matched by the binding `x`: `&'i str` (`&'i [u8]` with binary input) |
| `@x` | the span of the input matched by `x`: a `std::range::Range<usize>` of byte offsets |
| `#_` | the input matched by the whole sequence |
| `@_` | the span of the whole sequence |

They do not include whitespace before what was matched.

```rust
use std::range::Range;

#[derive(Debug, PartialEq)]
pub struct Assign<'a> {
    name: &'a str,
    name_span: Range<usize>,
    span: Range<usize>,
}

prpr::grammar! {
    pub assign: Assign<'i> = n:ident "=" ident {
        Assign { name: #n, name_span: @n, span: @_ }
    }

    atom ident = [a-z]+
}

fn main() {
    assert_eq!(
        assign("  foo = bar"),
        Ok(Assign { name: "foo", name_span: Range::from(2..5), span: Range::from(2..11) })
    );
}
```

A binding used only through `#x` or `@x`, like `n` above, has no value of its own, and `ident` needs no type.

`#` and `@` followed by a name are recognized in Rust code that is not in a `quote!` macro (where `#x` means interpolation), and not after an identifier, so Rust patterns like `n @ 1..=9` keep their meaning.

## Values that are not needed

prpr only builds values that are used.
When the value of an expression is not needed, because it is silent, not bound, only used through `#x`, or in a rule without a type, prpr generates code that only recognizes the input.
No vectors are built for its repetitions, and its actions do not run.

Actions should therefore not have side effects.
Besides, when the input is invalid, it is parsed a second time to find out what was expected (see [Error messages](errors.md#how-errors-are-found)), so actions may run twice.

## Semantic predicates

A sequence can end with `&{ code }` instead of an action.
The code decides whether the sequence matches:

| the code returns | the sequence |
|---|---|
| `bool` | matches if `true`; its value is `()` |
| `Option<T>` | matches if `Some(v)`; its value is `v` |
| `Result<T, E>` where `E: Display` | matches if `Ok(v)`; its value is `v`, and `Err(e)` is reported as what was expected |

```rust
prpr::grammar! {
    pub byte: u8 = s:digits &{ #s.parse::<u8>().map_err(|_| "a number up to 255") }

    pub even: u32 = s:digits &{ #s.parse::<u32>().ok().filter(|n| n % 2 == 0) }

    atom digits = [0-9]+
}

fn main() {
    assert_eq!(byte("255"), Ok(255));
    assert_eq!(
        byte("256").unwrap_err().to_string(),
        "expected a number up to 255, found `2`"
    );
    assert_eq!(even("42"), Ok(42));
    assert!(even("43").is_err());
}
```

Predicates always run, even when their values are not needed.
