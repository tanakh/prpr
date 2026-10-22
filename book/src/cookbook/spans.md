# Syntax trees with spans

Compilers and other tools report problems at places in the source, long after parsing.
For that, the nodes of the syntax tree keep their *spans*: the byte ranges of the input they were parsed from.

## Taking spans

In an action, `@x` is the span of the binding `x`, and `@_` the span of the whole sequence (see [The text and the span of a binding](../guide/values-and-actions.md#the-text-and-the-span-of-a-binding)).
Spans are `std::range::Range<usize>`, which is `Copy`, and they never include the whitespace before what was matched.

A generic wrapper gives a span to any value:

```rust
use std::range::Range;

#[derive(Debug, PartialEq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Range<usize>,
}

prpr::grammar! {
    pub assign: (Spanned<&'i str>, Spanned<i64>) = name:ident "=" value:number { (name, value) }

    atom ident: Spanned<&'i str> = s:([a-z]+) { Spanned { node: #s, span: @s } }

    atom number: Spanned<i64> = s:([0-9]+) { Spanned { node: #s.parse().unwrap(), span: @s } }
}

fn main() {
    let input = "  x = 42";
    let (name, value) = assign(input).unwrap();
    assert_eq!(name, Spanned { node: "x", span: Range::from(2..3) });
    assert_eq!(&input[value.span], "42");
}
```

A span slices the input directly, as `&input[value.span]` does.

## Spans of nodes built in Rust

Nodes that actions build from other nodes, such as binary operations folded from a repetition, take their spans from their parts:

```rust
use std::range::Range;

pub struct Expr {
    pub kind: Kind,
    pub span: Range<usize>,
}

pub enum Kind {
    Num(i64),
    Add(Box<Expr>, Box<Expr>),
}

fn add(first: Expr, rest: Vec<Expr>) -> Expr {
    rest.into_iter().fold(first, |l, r| Expr {
        span: Range { start: l.span.start, end: r.span.end },
        kind: Kind::Add(Box::new(l), Box::new(r)),
    })
}

prpr::grammar! {
    pub sum: Expr = l:number rs:("+" r:number { r })* { add(l, rs) }

    atom number: Expr = s:([0-9]+) { Expr { kind: Kind::Num(#s.parse().unwrap()), span: @_ } }
}

fn main() {
    let input = "1 + 2 + 3 ";
    let e = sum(input).unwrap();
    assert_eq!(&input[e.span], "1 + 2 + 3");
    let Kind::Add(l, _) = e.kind else { panic!() };
    assert_eq!(&input[l.span], "1 + 2");
}
```

## Reporting errors at spans

Errors found after parsing, such as a variable that is not defined, are reported at the span of the node they are about.
`prpr::runtime::Snippet` renders a message at a span the way [parse errors are rendered](../guide/errors.md#displaying-errors):

```rust
use prpr::runtime::Snippet;
use std::range::Range;

pub struct Spanned<T> {
    pub node: T,
    pub span: Range<usize>,
}

pub struct Let<'a> {
    pub name: &'a str,
    pub value: Vec<Spanned<Operand<'a>>>,
}

pub enum Operand<'a> {
    Num(i64),
    Var(&'a str),
}

prpr::grammar! {
    pub program: Vec<Let<'i>> = statement*

    statement: Let<'i> = "let" name:ident "=" value:(operand, "+")+ ";" { Let { name, value } }

    operand: Spanned<Operand<'i>>
        = n:number { Spanned { node: Operand::Num(n), span: @_ } }
        / v:ident { Spanned { node: Operand::Var(v), span: @_ } }

    atom ident: &'i str = s:([a-z]+) { #s }

    atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
}

// Checks that variables are defined before they are used.
fn check(program: &[Let]) -> Result<(), Spanned<String>> {
    let mut defined = Vec::new();
    for statement in program {
        for operand in &statement.value {
            if let Operand::Var(v) = operand.node
                && !defined.contains(&v)
            {
                let message = format!("cannot find variable `{v}`");
                return Err(Spanned { node: message, span: operand.span });
            }
        }
        defined.push(statement.name);
    }
    Ok(())
}

fn main() {
    let src = "let x = 1;\nlet y = x + z;\n";
    let program = program(src).unwrap();
    let e = check(&program).unwrap_err();
    let snippet = Snippet { level: "error", message: &e.node, src, origin: "main.txt", span: e.span };
    assert_eq!(
        snippet.to_string(),
        "\
error: cannot find variable `z`
 --> main.txt:2:13
  |
2 | let y = x + z;
  |             ^"
    );
}
```

`level` is the word before the message, such as `error` or `warning`.
To report errors with other libraries, such as those in [Error messages](../guide/errors.md#with-error-reporting-libraries), convert spans with `std::ops::Range::from(span)`.
`prpr::runtime::line_col(src, offset)` gives the line and the column of an offset, for messages of your own.

## Text or spans

A node can keep the text it was parsed from, as `&'i str`, its span, or both.
The text is convenient for names and makes the tree borrow the input; a span is small, does not borrow, and gives the text back with the input: `&input[span]`.
Trees that outlive the input, or that are built from several inputs, keep spans, together with an identifier of the input when there are several.
