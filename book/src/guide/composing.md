# Composing grammars

## Using the rules of another grammar

A grammar can use the rules of another grammar by their paths.
This makes it possible to share tokens between grammars, or to embed one language in another:

```rust
mod lexical {
    prpr::grammar! {
        pub atom ident: &'i str = s:([a-z]+) { #s }

        pub atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }

        pub list: Vec<i64> = "[" xs:(number, ",")* "]" { xs }
    }
}

mod config {
    #[derive(Debug, PartialEq)]
    pub enum Value {
        Number(i64),
        List(Vec<i64>),
    }

    prpr::grammar! {
        pub config: Vec<(&'i str, Value)> = entry*

        entry: (&'i str, Value) = k:super::lexical::ident "=" v:value ";" { (k, v) }

        value: Value
            = n:super::lexical::number { Value::Number(n) }
            / l:super::lexical::list { Value::List(l) }
    }
}

fn main() {
    use config::Value;
    assert_eq!(
        config::config("a = 1; b = [2, 3];"),
        Ok(vec![("a", Value::Number(1)), ("b", Value::List(vec![2, 3]))])
    );
    assert_eq!(
        config::config("a = 1x;").unwrap_err().to_string(),
        "expected delimiter, found `x`"
    );
}
```

A rule of another grammar behaves as it does in its own grammar:

* If it is an atom, it is a token, with the delimiters of its own grammar.
* If it is a syntactic rule, it skips whitespace as defined by its own grammar.
* What it expected is reported together with the errors of the other grammar.
* If its grammar memoizes rules, their tables are part of the `Memo` of the grammar that uses it.

The rule must be visible from where it is used, and both grammars must parse the same kind of input.
Since its type is not known when the code is generated, its value is never [silent](values-and-actions.md#the-values-of-expressions), even if it is `()`.

## Calling rules from Rust

The functions generated for rules, such as `config::config`, parse a whole input.
For more control, each grammar also has a type `Rules` with an associated function for each rule:

```rust,ignore
impl Rules {
    pub fn config<'i>(
        cx: &mut prpr::runtime::Cx<'i, str>,
        memo: &mut Memo<'i>,
        pos: usize,
    ) -> prpr::runtime::PResult<Vec<(&'i str, Value)>>;
    // ...
}
```

Such a function matches the rule at a byte position `pos` and returns its value and the position after it.
`prpr::runtime::parse_prefix` runs one on the start of an input:

```rust
mod number {
    prpr::grammar! {
        pub atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
    }
}

fn main() {
    let input = "42 and the rest";
    let (n, end) = prpr::runtime::parse_prefix(input, number::Rules::number).unwrap();
    assert_eq!((n, &input[end..]), (42, " and the rest"));
}
```

`prpr::runtime::parse_full` is what the generated functions use to parse a whole input.
