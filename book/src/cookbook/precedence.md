# Operators and precedence

PEGs have no notion of operator precedence, and they cannot express left recursion such as `expr = expr "+" term`.
Instead, precedence comes from layering rules, and associativity from how the values of repetitions are combined.
The examples in this chapter build syntax trees that print themselves with parentheses, to show how they are grouped:

```rust
# use std::fmt;
#
pub enum Expr {
    Num(i64),
    Unary(&'static str, Box<Expr>),
    Binary(Box<Expr>, &'static str, Box<Expr>),
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expr::Num(n) => write!(f, "{n}"),
            Expr::Unary(op, e) => write!(f, "({op}{e})"),
            Expr::Binary(l, op, r) => write!(f, "({l} {op} {r})"),
        }
    }
}
#
# fn main() {}
```

## A rule for each level

Each level of precedence is a rule that combines operands of the next tighter level, as in [the calculator](../getting-started/first-parser.md#operators-and-precedence).
A left-associative operator is a repetition whose values are folded from the left:

```rust
# use std::fmt;
# pub enum Expr {
#     Num(i64),
#     Unary(&'static str, Box<Expr>),
#     Binary(Box<Expr>, &'static str, Box<Expr>),
# }
# impl fmt::Display for Expr {
#     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
#         match self {
#             Expr::Num(n) => write!(f, "{n}"),
#             Expr::Unary(op, e) => write!(f, "({op}{e})"),
#             Expr::Binary(l, op, r) => write!(f, "({l} {op} {r})"),
#         }
#     }
# }
fn fold(first: Expr, rest: Vec<(&'static str, Expr)>) -> Expr {
    rest.into_iter().fold(first, |l, (op, r)| Expr::Binary(Box::new(l), op, Box::new(r)))
}

prpr::grammar! {
    pub expr: Expr = l:product rs:(("+" / "-") product)* { fold(l, rs) }

    product: Expr = l:power rs:(("*" / "/") power)* { fold(l, rs) }

    // `^` is right-associative: its right operand is again a power.
    power: Expr = l:unary r:("^" r:power { r })? {
        match r {
            Some(r) => Expr::Binary(Box::new(l), "^", Box::new(r)),
            None => l,
        }
    }

    // A prefix operator applies to an operand of its own level.
    unary: Expr = op:("-" / "!") e:unary { Expr::Unary(op, Box::new(e)) } / primary

    primary: Expr = n:number { Expr::Num(n) } / "(" e:expr ")" { e }

    #[label = "number"]
    atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
}

fn main() {
    let show = |s| expr(s).unwrap().to_string();
    assert_eq!(show("1 + 2 * 3 - 4"), "((1 + (2 * 3)) - 4)");
    assert_eq!(show("2 ^ 3 ^ 2"), "(2 ^ (3 ^ 2))");
    assert_eq!(show("-2 ^ 2"), "((-2) ^ 2)");
    assert_eq!(show("(1 + 2) * -3"), "((1 + 2) * (-3))");
}
```

* A right-associative operator calls its own rule for the right operand.
  Writing it as `l:unary "^" r:power { ... } / unary` would work too, but would parse `unary` twice when there is no `^`; making the rest optional parses it once.
* The tighter a level, the later it comes in the chain of rules.
  Here `-2 ^ 2` is `(-2) ^ 2`; for `-(2 ^ 2)`, as in mathematics, make `unary` call `power` instead of the other way around.

## Postfix operators

Postfix operators, such as function calls, indexing and field access, are a repetition after an operand, folded from the left:

```rust
# use std::fmt;
pub enum Expr {
    Var(String),
    Call(Box<Expr>, Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Field(Box<Expr>, String),
}

pub enum Postfix {
    Call(Vec<Expr>),
    Index(Expr),
    Field(String),
}

fn apply(e: Expr, op: Postfix) -> Expr {
    match op {
        Postfix::Call(args) => Expr::Call(Box::new(e), args),
        Postfix::Index(i) => Expr::Index(Box::new(e), Box::new(i)),
        Postfix::Field(name) => Expr::Field(Box::new(e), name),
    }
}
#
# impl fmt::Display for Expr {
#     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
#         match self {
#             Expr::Var(v) => write!(f, "{v}"),
#             Expr::Call(e, args) => {
#                 let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
#                 write!(f, "{e}({})", args.join(", "))
#             }
#             Expr::Index(e, i) => write!(f, "{e}[{i}]"),
#             Expr::Field(e, name) => write!(f, "({e}.{name})"),
#         }
#     }
# }

prpr::grammar! {
    pub postfix: Expr = e:primary ops:postfix_op* { ops.into_iter().fold(e, apply) }

    postfix_op: Postfix
        = "(" args:(postfix, ",")* ")" { Postfix::Call(args) }
        / "[" i:postfix "]" { Postfix::Index(i) }
        / "." name:ident { Postfix::Field(name.to_string()) }

    primary: Expr = name:ident { Expr::Var(name.to_string()) }

    atom ident: &'i str = s:([a-z]+) { #s }
}

fn main() {
    assert_eq!(postfix("a.b(c)[d].e").unwrap().to_string(), "((a.b)(c)[d].e)");
}
```

## Non-associative operators

When `a < b < c` should be an error, make the operator and its right operand optional instead of repeated:

```rust
prpr::grammar! {
    pub comparison: (i64, Option<(&'static str, i64)>) = number (("<=" / "<" / "==") number)?

    atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
}

fn main() {
    assert_eq!(comparison("1 <= 2"), Ok((1, Some(("<=", 2)))));
    assert!(comparison("1 < 2 < 3").is_err());
}
```

## Operators that start alike

Operators made of symbols are not checked for [delimiters](../guide/whitespace.md#delimiters), since they do not end with a word character, so `"<"` matches the start of `<=`.
In a choice, list longer operators before the operators they start with, as the example above does; otherwise `<` would match and leave `=` where an operand should be.

Operators on different levels need no such care, because an operator must be followed by its operand.
If `|` binds tighter than `||`, the `|` level matches the first character of `||`, fails to find an operand at the second one (as long as no operand starts with `|`), and gives the `|` back to the `||` level.

## Many levels

With many levels, a rule for each level gets long, and each level costs a call for every operand.
Another way is to parse a flat list of operands and binary operators, and to build the tree with a table of precedences in Rust, by [precedence climbing](https://en.wikipedia.org/wiki/Operator-precedence_parser#Precedence_climbing_method):

```rust
# use std::fmt;
# pub enum Expr {
#     Num(i64),
#     Unary(&'static str, Box<Expr>),
#     Binary(Box<Expr>, &'static str, Box<Expr>),
# }
# impl fmt::Display for Expr {
#     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
#         match self {
#             Expr::Num(n) => write!(f, "{n}"),
#             Expr::Unary(op, e) => write!(f, "({op}{e})"),
#             Expr::Binary(l, op, r) => write!(f, "({l} {op} {r})"),
#         }
#     }
# }
// The precedence of binary operators, and whether they are right-associative.
fn precedence(op: &str) -> (u8, bool) {
    match op {
        "||" => (1, false),
        "&&" => (2, false),
        "==" | "!=" | "<" | "<=" | ">" | ">=" => (3, false),
        "+" | "-" => (4, false),
        "*" | "/" | "%" => (5, false),
        "^" => (6, true),
        _ => unreachable!(),
    }
}

// Builds the tree of `first op1 e1 op2 e2 ...`.
fn climb(first: Expr, rest: Vec<(&'static str, Expr)>) -> Expr {
    let mut rest = rest.into_iter().peekable();
    climb_from(first, &mut rest, 0)
}

// Takes the operators of precedence `min` or higher from `rest` and combines them with `lhs`.
fn climb_from(
    mut lhs: Expr,
    rest: &mut std::iter::Peekable<std::vec::IntoIter<(&'static str, Expr)>>,
    min: u8,
) -> Expr {
    while let Some((op, mut rhs)) = rest.next_if(|(op, _)| precedence(op).0 >= min) {
        let (prec, _) = precedence(op);
        // Operators that bind tighter than `op` take `rhs` first.
        while let Some((next, _)) = rest.peek() {
            let (next_prec, right) = precedence(next);
            if next_prec > prec || (next_prec == prec && right) {
                rhs = climb_from(rhs, rest, next_prec);
            } else {
                break;
            }
        }
        lhs = Expr::Binary(Box::new(lhs), op, Box::new(rhs));
    }
    lhs
}

prpr::grammar! {
    pub expr: Expr = first:unary rest:(binary_op unary)* { climb(first, rest) }

    binary_op: &'static str
        = "||" / "&&" / "==" / "!=" / "<=" / ">=" / "<" / ">"
        / "+" / "-" / "*" / "/" / "%" / "^"

    unary: Expr = op:("-" / "!") e:unary { Expr::Unary(op, Box::new(e)) } / primary

    primary: Expr = n:number { Expr::Num(n) } / "(" e:expr ")" { e }

    atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
}

fn main() {
    let show = |s| expr(s).unwrap().to_string();
    assert_eq!(show("1 + 2 * 3 == 7 && !0"), "(((1 + (2 * 3)) == 7) && (!0))");
    assert_eq!(show("1 - 2 - 3"), "((1 - 2) - 3)");
    assert_eq!(show("2 ^ 3 ^ 2 * 4"), "((2 ^ (3 ^ 2)) * 4)");
}
```

The value of `binary_op unary` is a pair of the operator, as the literal that matched, and the operand, so `rest` is a `Vec<(&'static str, Expr)>`.
Adding an operator is a matter of adding it to `binary_op` and to the table.
