# Grammar syntax

This chapter summarizes the syntax of grammars, as written inside `grammar!` or in a grammar file.
The Guide explains what each part means; [The grammar of grammars](prpr-peg.md) defines the syntax exactly.

## Lexical structure

* **Whitespace** separates tokens and is otherwise ignored.
* **Comments** are those of Rust: `// ...` to the end of the line, and `/* ... */`, which may be nested.
  `/// ...` is a documentation comment, which is an attribute of the rule after it.
* **Identifiers** are those of Rust, including non-ASCII letters.
  Rust keywords, and `atom`, cannot be rule names or binding names.
* **Literals** are Rust string and character literals; see [Literals](#literals).

## Grammars

```text
grammar     = grammar-attribute* item*
item        = rust-item / rule
```

Grammar attributes come first:

| attribute | meaning |
|---|---|
| `#![input = str]` | The input is `&str` (the default). |
| `#![input = bytes]` | The input is `&[u8]`. See [Binary input](../guide/binary-input.md). |
| `#![lint]` | Check the grammar for common mistakes. See [Lints](../guide/lints.md). |
| `#![profile]` | Count the calls of rules. See [Finding what to memoize](../guide/memoization.md#finding-what-to-memoize). |

## Rust items

Rust items are copied to the generated code as they are.
They may have attributes, doc comments and a visibility, and start with one of:

| keyword | ends at |
|---|---|
| `use`, `static`, `type`, `extern crate`, and `const` items | `;` |
| `fn`, `struct`, `enum`, `union`, `impl`, `trait`, `mod`, `macro_rules!`, `unsafe`, `async`, `extern`, `const fn` | `;` or the end of their `{ ... }` block, whichever comes first |

Other items, such as macro invocations other than `macro_rules!`, are not recognized; put them outside the grammar.

## Rules

```text
rule        = attribute* visibility? "atom"? name (":" type)? "=" expression
```

* `visibility` is a Rust visibility: `pub`, `pub(crate)`, `pub(super)` or `pub(in path)`.
* `type` is a Rust type: a path with generic arguments, a reference, a tuple, an array or a slice, a pointer, `dyn` bounds or a function pointer.
  Qualified paths such as `<T as Trait>::Output` are not supported, and `impl` bounds, `!` and `_` are parsed but cannot be the types of rules.
* A rule ends where the next rule or Rust item begins: the body of a rule stops before `name =` or `name: type =`.

| attribute | meaning |
|---|---|
| `/// ...`, `#[doc = "..."]` | Documentation of the generated function. |
| `#[label = "..."]` | The name of the rule in error messages. See [Labels](../guide/errors.md#labels). |
| `#[memo]` | Memoize the rule. See [Memoization](../guide/memoization.md). |
| `#[no_memo]` | Do not memoize the rule, on purpose, which [`#![lint]`](../guide/lints.md) takes into account. |
| `#[boundary(expression)]` | On an atom: what must follow the token. See [Delimiters](../guide/whitespace.md#delimiters). |
| `#[no_boundary]` | On an atom: nothing needs to follow the token. |

## Expressions

From the loosest to the tightest binding:

```text
expression  = sequence ("/" sequence)*
sequence    = element* action?
action      = "{" rust-code "}"
            / "&{" rust-code "}"
element     = name ":" prefix
            / prefix
prefix      = "&" prefix
            / "!" prefix
            / postfix
postfix     = primary ("*" / "+" / "?")?
            / "(" expression "," expression ")" ("*" / "+")
primary     = "(" expression ")"
            / "<" expression ">"
            / literal
            / class
            / "."
            / path
path        = "::"? segment ("::" segment)*
```

| expression | meaning | see |
|---|---|---|
| `e1 / e2` | ordered choice | [Ordered choice](../guide/expressions.md#ordered-choice) |
| `e1 e2` | sequence | [Sequences](../guide/expressions.md#sequences) |
| `e { code }` | action | [Actions](../guide/values-and-actions.md#actions) |
| `e &{ code }` | semantic predicate | [Semantic predicates](../guide/values-and-actions.md#semantic-predicates) |
| `name:e` | binding | [Actions](../guide/values-and-actions.md#actions) |
| `&e`, `!e` | lookahead | [Lookahead](../guide/expressions.md#lookahead) |
| `e*`, `e+`, `e?` | repetition, option | [Repetition](../guide/expressions.md#repetition) |
| `(e, sep)*`, `(e, sep)+` | separated list | [Separated lists](../guide/expressions.md#separated-lists) |
| `<e>` | atomic group | [Atomic groups](../guide/expressions.md#atomic-groups) |
| `"abc"`, `'c'` | literal | [Literals](../guide/expressions.md#literals) |
| `[a-z]`, `[^a-z]` | character class | [Character classes](../guide/expressions.md#character-classes) |
| `.` | any character | [Any character](../guide/expressions.md#any-character) |
| `rule`, `path::to::rule` | rule reference | [Composing grammars](../guide/composing.md) |

* A segment of a path is an identifier, or `crate`, `super` or `self`.
* An action or a predicate ends its sequence.
  An action may also make up a sequence on its own, as in `x / { 0 }`.
* `&` directly followed by `{` starts a predicate, not a lookahead.

## Literals

| literal | example | input |
|---|---|---|
| string | `"if"`, `"\n"`, `"\u{3042}"` | text and bytes (as UTF-8) |
| raw string, with up to three `#`s | `r"\d"`, `r#"say "hi""#` | text and bytes |
| character | `'a'`, `'\''` | text and bytes (as UTF-8) |
| byte string | `b"\x89PNG"` | bytes |
| raw byte string | `br"\x"` | bytes |
| byte | `b'\0'` | bytes |

Escapes are those of Rust: `\n`, `\r`, `\t`, `\\`, `\0`, `\'`, `\"`, `\x7F` (up to `\x7F` in strings, any byte in byte strings), `\u{...}` (not in byte strings), and a backslash at the end of a line, which skips the line break and the whitespace after it.

## Character classes

```text
class       = "[" "^"? (item ("-" item)?)* "]"
item        = character / character-literal / byte-literal
```

* A character stands for itself, except whitespace, `[`, `]`, `'`, `"` and `\`, which must be written as character literals such as `' '` or `'\\'`.
* `a-b` is a range, which must be in ascending order.
  A `-` that is not between two items stands for itself.
* `^` right after `[` negates the class.
* `/` must be written as `'/'` where it would start a comment (`//` or `/*`).
* Inside `grammar!`, a digit directly followed by `e` or `E` is read by Rust as part of a number, so write `[0-9 e]` rather than `[0-9e]`.

## Rust code in actions

Rust code in actions and predicates is ordinary Rust code, with these additions:

| | is |
|---|---|
| `#x` | the input matched by the binding `x` |
| `@x` | the span of the input matched by `x` |
| `#_` | the input matched by the sequence |
| `@_` | the span of the sequence |

`#` and `@` are recognized before an identifier or `_`, except inside the `quote!` macro and after an identifier (as in the pattern `n @ 1..=9`).
See [The text and the span of a binding](../guide/values-and-actions.md#the-text-and-the-span-of-a-binding).
