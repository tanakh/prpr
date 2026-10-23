# Memoization and performance

## Backtracking

When an alternative of a choice fails, the next alternative starts again from the same position.
Rules that the first alternative already ran may then run again at the same position.
Usually this costs little, but when it happens at every level of a nested structure, the work doubles at each level:

```rust,ignore
prpr::grammar! {
    // `term` is parsed twice when there is no `!`, at every level of nesting.
    pub term: i64
        = "(" t:term ")" "!" { t + 1 }
        / "(" t:term ")" { t }
        / "x" { 0 }
}
```

With 30 nested parentheses and no `!`, this grammar runs `term` about a billion times.

## `#[memo]`

A Packrat parser avoids this by remembering the result of each rule at each position.
In prpr, a rule remembers its results when it has the `#[memo]` attribute:

```rust
prpr::grammar! {
    #[memo]
    pub term: i64
        = "(" t:term ")" "!" { t + 1 }
        / "(" t:term ")" { t }
        / "x" { 0 }
}

fn main() {
    let input = format!("{}x{}", "(".repeat(1000), ")".repeat(1000));
    assert_eq!(term(&input), Ok(0));
}
```

The results are kept in a table for each memoized rule, in the `Memo` struct that each parse creates and drops; there is no global state.

When a memoized rule is invoked again at the same position, it returns a clone of its value, so the type of a memoized rule must implement `Clone`.
If cloning would be expensive, wrap the value in an `Rc`.
A type that does not implement `Clone` is reported at the type of the rule.

Memoization has a cost: every invocation of the rule looks up and stores a result.
That is why prpr memoizes only the rules that ask for it.

## When to memoize

A rule is worth memoizing when several alternatives can invoke it at the same position, typically because they start the same way, and when it is not cheap: a syntactic rule rather than a token.

Often, factoring out the common start is even better than memoizing:

```rust
prpr::grammar! {
    pub term: i64
        = "(" t:term ")" bang:"!"? { if bang.is_some() { t + 1 } else { t } }
        / "x" { 0 }
}

fn main() {
    assert_eq!(term("((x)!)"), Ok(1));
}
```

## Finding what to memoize

Two tools help find the rules that are parsed more than once.
[`#![lint]`](lints.md#rules-parsed-more-than-once) finds rules that alternatives may parse at the same position, from the grammar alone.
Profiling counts how often each rule is actually parsed again on an input.

With `#![profile]`, the rules of a grammar count their calls, and `prpr::runtime::profile` runs one of them from the start of an input, as `parse_prefix` does, and returns the counts with the result:

```rust
mod calc {
    prpr::grammar! {
        #![profile]

        pub expr: i64 = l:term rs:(("+" / "-") term)* {
            rs.into_iter().fold(l, |a, (op, b)| if op == "+" { a + b } else { a - b })
        }

        term: i64 = l:factor rs:(("*" / "/") factor)* {
            rs.into_iter().fold(l, |a, (op, b)| if op == "*" { a * b } else { a / b })
        }

        factor: i64 = "(" e:expr ")" "!" { e * 2 } / "(" e:expr ")" { e } / number

        atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
    }
}

fn main() {
    let (result, profile) = prpr::runtime::profile("((1 + 2) * 3) - 4", calc::Rules::expr);
    assert_eq!(result, Ok((5, 17)));
    println!("{profile}");
#     assert_eq!(profile.rules().iter().find(|r| r.rule == "factor").unwrap().reparsed(), 8);
}
```

```text
rule         calls    repeated   memo hits    reparsed
factor          14           8           0           8
term            12           7           0           7
number          11           7           0           7
expr             7           4           0           4
```

* `calls` counts the calls of the rule, and `repeated` those at a position where the rule had already been called.
* `memo hits` counts the repeated calls that returned a memoized result, and `reparsed` the others, which parsed the input again.

Rules are listed with the most reparsed first.
Here, the first two alternatives of `factor` both parse `"(" expr ")"`, so every parenthesized expression without `!` is parsed twice, with everything in it.
Memoizing `factor` or `expr`, or factoring out `"(" e:expr ")"`, removes the reparsing.

`profile.rules()` gives the counts as values, for tests or tools.

The counts are kept in the `Memo` of the grammar, so grammars without `#![profile]` pay nothing.
In turn, `profile` only takes the rules of a grammar with `#![profile]`, and counts the rules of that grammar only, not those of [other grammars](composing.md) it uses:

```rust,compile_fail
mod uncounted {
    prpr::grammar! {
        pub x = "x"
    }
}

fn main() {
    // error: the grammar of `Memo<'_>` does not count the calls of its rules
    let _ = prpr::runtime::profile("x", uncounted::Rules::x);
}
```

A grammar with `#![profile]` counts the calls of its rules whenever they run, which makes it slower, so remove the attribute when you are done.

## Writing fast grammars

prpr generates straight-line Rust code, and a few habits help it generate fast code:

* **Make tokens atoms.**
  Inside atoms nothing is skipped, and repetitions of character classes that contain only ASCII characters, such as `[0-9]+` or `[^ '"' '\\']*`, compile to loops over bytes when their values are not needed.
* **Take slices instead of building values.**
  `s:([0-9]+) { #s.parse().unwrap() }` borrows the digits from the input, while `ds:[0-9]+` would build a `Vec<char>`.
  Values that are not needed are not built: rules without a type and unbound elements only recognize the input.
* **Borrow from the input.**
  Types with the lifetime `'i`, such as `&'i str`, avoid copying, as in [the JSON tutorial](../getting-started/json.md#step-6-borrowing-from-the-input).
* **Let alternatives fail early.**
  Alternatives that start with different tokens fail at their first token, which is cheap.

Error reporting does not slow down parsing of valid input, because failures are only recorded when a parse is repeated after it failed (see [Error messages](errors.md#how-errors-are-found)).

On the JSON documents of the [benchmarks](https://github.com/tanakh/prpr/tree/master/benchmarks), prpr parses faster than rust-peg, pest, nom and winnow, and close to serde_json:

| document | prpr | rust-peg | pest | nom | winnow | serde_json |
|---|---:|---:|---:|---:|---:|---:|
| canada.json (2.2 MB) | 8.0 ms | 8.9 ms | 35.7 ms | 12.8 ms | 14.7 ms | 4.6 ms |
| citm_catalog.json (1.7 MB) | 2.4 ms | 3.0 ms | 28.6 ms | 4.7 ms | 6.1 ms | 2.7 ms |
| twitter.json (0.6 MB) | 1.4 ms | 1.6 ms | 14.1 ms | 2.2 ms | 3.0 ms | 1.3 ms |
