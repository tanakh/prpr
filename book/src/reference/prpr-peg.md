# The grammar of grammars

prpr parses grammars with a parser that prpr generates from a grammar, written in prpr itself.
That grammar, `prpr-codegen/src/parser/prpr.peg`, is reproduced below.
It is the exact definition of the syntax summarized in [Grammar syntax](syntax.md), and an example of a grammar of real size.

Some techniques it uses:

* **Rules without terminators.**
  A sequence stops before what looks like the start of the next rule or Rust item, with `!item_start` before each element.
* **Rust code without a Rust parser.**
  Actions, types and Rust items are only matched as far as needed to find where they end: Rust code is a series of token trees (`rust_tt`), with balanced brackets, and with strings, characters and comments that may contain brackets.
* **Comments that are attributes.**
  `///` starts a documentation comment, which is an attribute of the rule after it, while `//` and `////` start ordinary comments.
* **Identifiers and keywords.**
  `ident_raw` matches identifiers of any script, and `ident` excludes keywords with a predicate, as in [Identifiers and keywords](../cookbook/keywords.md).
* **Checks with messages.**
  Predicates check what expressions alone cannot, such as that a `\x` escape in a string is at most `\x7F`, and report what was expected when the check fails.

The parser is generated with the runtime included, so that changes to the runtime cannot break it.
After changing the grammar, regenerate the parser with `PRPR_BLESS=1 cargo test -p prpr-cli --test bootstrap`; the same test checks that the parser is up to date.

```rust,ignore
{{#include ../../../prpr-codegen/src/parser/prpr.peg}}
```
