# Diagnostics

prpr checks a grammar before it generates code from it.
This chapter lists the errors it reports, and the errors in grammars that the Rust compiler finds in the generated code.

## How errors are shown

With `grammar!`, errors are compile errors at the tokens of the grammar they are about.
Notes are part of the message, after it:

```text
error: cannot find rule `factor`
       = note: a rule with a similar name exists: `fact`
 --> src/main.rs:3:12
  |
3 |     term = factor
  |            ^^^^^^
```

For grammar files, `prpr check`, `prpr gen` and `prpr_codegen::file::generate` render errors with the line of the grammar they are about:

```text
error: cannot find rule `factor`
 --> grammar.peg:1:8
  |
1 | term = factor
  |        ^^^^^^
  = note: a rule with a similar name exists: `fact`
```

## Syntax errors

**`syntax error: expected ..., found ...`**

The grammar does not follow the [syntax](syntax.md).
The error says what could come where the grammar stopped making sense:

```text
error: syntax error: expected expression, found `/`
 --> grammar.peg:1:11
  |
1 | a = "x" / / "y"
  |           ^
```

Inside `grammar!`, the Rust compiler reads the grammar as Rust tokens first, and reports what is not a Rust token, such as an unterminated string literal or `[0-9e]` (see [Character classes](../guide/expressions.md#character-classes)).

## Rules

**`` cannot find rule `name` ``**

The grammar refers to a rule that it does not define.
A note suggests a rule with a similar name, if there is one.
To use a rule of another grammar, write its path, such as `lexer::ident`.

**`a reference to a rule of another grammar needs the path of its module`**

A path like `::ident` names no module.
Write the path of the module of the grammar, such as `crate::lexer::ident`.

**`` rule `name` is defined more than once ``**

Two rules have the same name.
A note points at the first definition.

**`` `match` is a keyword and cannot be a rule name ``**

Rule names become the names of Rust functions, so they cannot be Rust keywords; `atom` cannot be one either.

**`` `comment` must be an `atom` rule ``**

`whitespace`, `comment`, `spaces` and `word_char` are used to skip whitespace and check delimiters, which must not skip whitespace themselves, so a grammar that defines them must define them as atoms.

## Attributes

**`` unknown attribute `name` ``**, **`` unknown grammar attribute `name` ``**

The attributes of rules are listed in [Rules](../guide/rules.md#attributes), and the only grammar attribute is `#![input = ...]`.

**`` expected `#[label = "..."]` ``**, **`` expected `#[memo]` without arguments ``**, **`` expected `#[boundary(expression)]` ``**, **`` expected `#![input = str]` or `#![input = bytes]` ``**

An attribute has the wrong kind of argument.

**`` `#[boundary]` and `#[no_boundary]` only apply to `atom` rules ``**

Only tokens are checked for delimiters, and syntactic rules are not tokens.
Make the rule an atom, or put the expression in the rule itself, as a lookahead at its end.

## Expressions

**`` byte literals require `#![input = bytes]` ``**

Byte strings such as `b"\x89PNG"` and bytes such as `b'\0'` are only allowed in grammars of [binary input](../guide/binary-input.md).

**`` a character class for byte input can only contain characters up to `\u{ff}` ``**

In a grammar of binary input, a class matches a byte, and its characters stand for the bytes of the same values.

**`` `name` is bound more than once in this sequence ``**

Two elements of a sequence have the same binding name.
Rename one of them.
Sequences in parentheses have bindings of their own, so the same name may be used inside and outside of them.

**`repetition of an expression that can match empty input`**

A repetition such as `("a"?)*` would repeat forever without consuming input.
Repeat the part that consumes input, as in `"a"*`.

**`` left recursion: `expr` -> `expr` ``**

A rule can invoke itself, directly or through other rules (the error lists them), without consuming any input, as in `expr = expr "+" term / term`.
PEG parsers cannot parse such rules; use repetitions instead, as explained in [Operators and precedence](../cookbook/precedence.md).

## Rust code

**`` `#name` does not refer to a binding of this sequence ``**

`#name` and `@name` in an action refer to bindings of the sequence of the action.
Bindings of other sequences, such as sequences in parentheses, are not visible.

**`invalid Rust code: ...`**

In a grammar file, the Rust code in an action, a type or a Rust item is not valid Rust syntax.
With `grammar!`, the Rust compiler reports such errors itself.

## Errors from the Rust compiler

The Rust code of a grammar is checked by the Rust compiler, in the generated code.
With `grammar!`, it reports errors at the grammar; for grammar files, it reports them in the generated module.

* **Type errors in actions** are reported at the action, and the type of the rule is pointed out as the expected type:

  ```text
  error[E0308]: mismatched types
   --> src/main.rs:2:30
    |
  2 |     num: i64 = s:([0-9]+) { #s.len() }
    |          ---                 ^ expected `i64`, found `usize`
    |          |
    |          expected `i64` because of return type
  ```

* **Memoized rules whose types are not `Clone`** are reported at the type of the rule:

  ```text
  error[E0277]: the trait bound `Tree: Clone` is not satisfied
   --> src/main.rs:5:11
    |
  5 |     tree: Tree = "x" { Tree }
    |           ^^^^ the trait `Clone` is not implemented for `Tree`
  ```

* **Rules of other grammars** that do not exist or are not visible are reported at their names, as errors that their grammar's `Rules` has no associated function of that name, or that it is private.
  The compiler may report several errors for one mistake, about helpers whose names start with `__`.
  A rule of a grammar of a different kind of input, text or binary, is reported as mismatched types between `Cx<'_, str>` and `Cx<'_, [u8]>`.
