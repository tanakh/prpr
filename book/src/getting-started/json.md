# Tutorial: a JSON parser

In this chapter, we write a parser for [JSON](https://www.json.org/), step by step.
Along the way, we meet separated lists, character classes, tokens with escapes, semantic predicates, error labels and values that borrow from the input.

The parser builds values of this type:

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}
# fn main() {}
```

We put the grammar in a module of its own, `json`.
`grammar!` generates functions and a few other items in the scope where it is invoked, so a module keeps them apart from the rest of the program.
Rust items in a grammar, such as the `use` below, are copied to that scope as they are.

## Step 1: literals and arrays

```rust
# #[derive(Clone, Debug, PartialEq)]
# pub enum Value {
#     Null,
#     Bool(bool),
#     Number(f64),
#     String(String),
#     Array(Vec<Value>),
#     Object(Vec<(String, Value)>),
# }
mod json {
    prpr::grammar! {
        use super::Value;

        pub value: Value
            = array
            / "true" { Value::Bool(true) }
            / "false" { Value::Bool(false) }
            / "null" { Value::Null }

        array: Value = "[" es:(value, ",")* "]" { Value::Array(es) }
    }
}

fn main() {
    use Value::*;
    assert_eq!(
        json::value("[true, [null], []]"),
        Ok(Array(vec![Bool(true), Array(vec![Null]), Array(vec![])]))
    );
    assert!(json::value("[true,]").is_err());
}
```

`(value, ",")*` is a *separated list*: zero or more values separated by commas.
Its value is a `Vec<Value>`.
A trailing comma is not allowed, as JSON requires; `(value, ",")+` would require at least one value.

Each alternative of `value` has an action that builds the value, except `array`, whose value is already a `Value`.

## Step 2: numbers

A JSON number has an optional minus sign, an integer part without leading zeros, an optional fraction and an optional exponent.
That translates directly into a rule:

```rust
# #[derive(Clone, Debug, PartialEq)]
# pub enum Value {
#     Null,
#     Bool(bool),
#     Number(f64),
#     String(String),
#     Array(Vec<Value>),
#     Object(Vec<(String, Value)>),
# }
mod json {
    prpr::grammar! {
        use super::Value;

        pub value: Value
            = array
            / n:number { Value::Number(n) }
            / "true" { Value::Bool(true) }
            / "false" { Value::Bool(false) }
            / "null" { Value::Null }

        array: Value = "[" es:(value, ",")* "]" { Value::Array(es) }

        atom number: f64 = s:("-"? ("0" / [1-9] [0-9]*) ("." [0-9]+)? ([eE] [+-]? [0-9]+)?) {
            #s.parse().unwrap()
        }
    }
}

fn main() {
    assert_eq!(json::value("-1.5e3"), Ok(Value::Number(-1500.0)));
    assert!(json::value("0123").is_err());
    assert!(json::value("1.").is_err());
}
```

`?` makes what precedes it optional.
`[eE]` and `[+-]` are character classes; inside the brackets, characters stand for themselves and `a-b` is a range.
`s` is bound to the whole group, so `#s` is the text of the entire number, which `str::parse` turns into an `f64`.

`number` is an `atom`, so no whitespace may appear inside a number.
`0123` is rejected because the token `0` is directly followed by `1`: tokens that end with a letter, digit or `_` must not be followed by another one.

## Step 3: strings

Strings are tokens too, with escape sequences:

```rust
# #[derive(Clone, Debug, PartialEq)]
# pub enum Value {
#     Null,
#     Bool(bool),
#     Number(f64),
#     String(String),
#     Array(Vec<Value>),
#     Object(Vec<(String, Value)>),
# }
mod json {
    prpr::grammar! {
        use super::Value;

        pub value: Value
            = array
            / s:string { Value::String(s) }
            / n:number { Value::Number(n) }
            / "true" { Value::Bool(true) }
            / "false" { Value::Bool(false) }
            / "null" { Value::Null }

        array: Value = "[" es:(value, ",")* "]" { Value::Array(es) }

        atom string: String = '"' cs:character* '"' { cs.into_iter().collect() }

        atom character: char
            = [^ '"' '\\' '\0'-'\x1F']
            / "\\" c:escape { c }

        atom escape: char
            = '"' { '"' }
            / "\\" { '\\' }
            / "/" { '/' }
            / "b" { '\x08' }
            / "f" { '\x0c' }
            / "n" { '\n' }
            / "r" { '\r' }
            / "t" { '\t' }
            / "u" h:(hex hex hex hex) &{ u32::from_str_radix(#h, 16).ok().and_then(char::from_u32) }

        atom hex = [0-9a-fA-F]
#
#         atom number: f64 = s:("-"? ("0" / [1-9] [0-9]*) ("." [0-9]+)? ([eE] [+-]? [0-9]+)?) {
#             #s.parse().unwrap()
#         }
    }
}

fn main() {
    assert_eq!(
        json::value(r#""tab\t, cat \u732b""#),
        Ok(Value::String("tab\t, cat 猫".to_string()))
    );
    assert!(json::value(r#""\x""#).is_err());
}
```

There are a few new things here:

* `'"'` is a literal of one character, written as a Rust character literal.
  Outside a class, `"\""` would mean the same.
* `[^ ... ]` is a negated class: any character except the listed ones.
  A class contains only characters: characters written as they are, such as `a` or `+`, and Rust character literals, such as `'"'`.
  String literals cannot appear in a class.
  Characters that would confuse Rust, such as quotes and backslashes, must be written as character literals, and `'\0'-'\x1F'` is a range of control characters.
  Whitespace inside a class is ignored, so `[a-z A-Z]` is the same as `[a-zA-Z]`.
* The value of a character class is the matched `char`, so `character*` is a `Vec<char>`.
* `hex` has no type.
  A rule without a type only recognizes the input: it checks that the input matches, and its value is `()`.
* In `"u" h:(hex hex hex hex)`, `h` is bound to the four hexadecimal digits.
  Their values are all `()`, but `#h`, in the code at the end of the sequence, is the text that `h` matched: a `&str` such as `"732b"`.
  `#` gives the text of any binding, whatever the type of its value.
* `&{ ... }` is a *semantic predicate*: its code returns an `Option`, and `None` makes the match fail.
  Here it turns the text of the digits into a character, and rejects `\u` escapes that are not valid characters.
  (A complete parser would also combine surrogate pairs such as `\ud83d\ude00`.)

## Step 4: objects

Objects are separated lists of members:

```rust
# #[derive(Clone, Debug, PartialEq)]
# pub enum Value {
#     Null,
#     Bool(bool),
#     Number(f64),
#     String(String),
#     Array(Vec<Value>),
#     Object(Vec<(String, Value)>),
# }
mod json {
    prpr::grammar! {
        use super::Value;

        pub value: Value
            = object
            / array
            / s:string { Value::String(s) }
            / n:number { Value::Number(n) }
            / "true" { Value::Bool(true) }
            / "false" { Value::Bool(false) }
            / "null" { Value::Null }

        object: Value = "{" kvs:(member, ",")* "}" { Value::Object(kvs) }

        member: (String, Value) = k:string ":" v:value { (k, v) }

        array: Value = "[" es:(value, ",")* "]" { Value::Array(es) }
#
#         atom string: String = '"' cs:character* '"' { cs.into_iter().collect() }
#
#         atom character: char
#             = [^ '"' '\\' '\0'-'\x1F']
#             / "\\" c:escape { c }
#
#         atom escape: char
#             = '"' { '"' }
#             / "\\" { '\\' }
#             / "/" { '/' }
#             / "b" { '\x08' }
#             / "f" { '\x0c' }
#             / "n" { '\n' }
#             / "r" { '\r' }
#             / "t" { '\t' }
#             / "u" h:(hex hex hex hex) &{ u32::from_str_radix(#h, 16).ok().and_then(char::from_u32) }
#
#         atom hex = [0-9a-fA-F]
#
#         atom number: f64 = s:("-"? ("0" / [1-9] [0-9]*) ("." [0-9]+)? ([eE] [+-]? [0-9]+)?) {
#             #s.parse().unwrap()
#         }
    }
}

fn main() {
    use Value::*;
    assert_eq!(
        json::value(r#"{"a": [1, "b"], "c": {}}"#),
        Ok(Object(vec![
            ("a".to_string(), Array(vec![Number(1.0), String("b".to_string())])),
            ("c".to_string(), Object(vec![])),
        ]))
    );
}
```

`member` has a tuple type and builds a tuple; `kvs` is a `Vec<(String, Value)>`.
That completes the parser: it accepts all of JSON.

## Step 5: better error messages

Let us see how the parser reports errors.
Without further work, the messages list everything the parser tried:

```text
[1, ]     expected one of `"`, `-`, `0`, `[`, `false`, `null`, `true`, `{`, or `[1-9]`, found `]`
["abc     expected one of `"`, `\\`, or `[^ '"' '\\' '\0'-'\x1F']`, found end of input
```

That is accurate, but not how people think about JSON.
*Labels* name what a rule stands for.
When a labeled rule fails where it starts, the error says its label instead of what was expected inside it:

```rust
# #[derive(Clone, Debug, PartialEq)]
# pub enum Value {
#     Null,
#     Bool(bool),
#     Number(f64),
#     String(String),
#     Array(Vec<Value>),
#     Object(Vec<(String, Value)>),
# }
mod json {
    prpr::grammar! {
        use super::Value;

        #[label = "value"]
        pub value: Value
            = object
            / array
            / s:string { Value::String(s) }
            / n:number { Value::Number(n) }
            / "true" { Value::Bool(true) }
            / "false" { Value::Bool(false) }
            / "null" { Value::Null }
#
#         object: Value = "{" kvs:(member, ",")* "}" { Value::Object(kvs) }
#
#         member: (String, Value) = k:string ":" v:value { (k, v) }
#
#         array: Value = "[" es:(value, ",")* "]" { Value::Array(es) }

        #[label = "string"]
        atom string: String = '"' cs:character* '"' { cs.into_iter().collect() }

        #[label = "character"]
        atom character: char
            = [^ '"' '\\' '\0'-'\x1F']
            / "\\" c:escape { c }
#
#         atom escape: char
#             = '"' { '"' }
#             / "\\" { '\\' }
#             / "/" { '/' }
#             / "b" { '\x08' }
#             / "f" { '\x0c' }
#             / "n" { '\n' }
#             / "r" { '\r' }
#             / "t" { '\t' }
#             / "u" h:(hex hex hex hex) &{ u32::from_str_radix(#h, 16).ok().and_then(char::from_u32) }
#
#         atom hex = [0-9a-fA-F]

        #[label = "number"]
        atom number: f64 = s:("-"? ("0" / [1-9] [0-9]*) ("." [0-9]+)? ([eE] [+-]? [0-9]+)?) {
            #s.parse().unwrap()
        }
    }
}

fn main() {
    let message = |s| json::value(s).unwrap_err().to_string();
    assert_eq!(message("[1, ]"), "expected value, found `]`");
    assert_eq!(message(r#"["abc"#), r#"expected one of `"` or character, found end of input"#);
    assert_eq!(message(r#"{"a": tru}"#), "expected value, found `t`");
    assert_eq!(message("{1: 2}"), "expected one of `}` or string, found `1`");
    assert_eq!(message("[1 2]"), "expected one of `,` or `]`, found `2`");
    assert_eq!(message("0123"), "expected delimiter, found `1`");
}
```

Labels only replace what was expected at the position where the rule starts.
Errors further inside a rule, such as a missing `]` after the elements of an array, are still reported as they are.

## Step 6: borrowing from the input

The parser copies every string into a `String`.
Most strings in JSON documents have no escapes, and they can borrow their text from the input instead.
The types of rules may use the lifetime `'i` of the input:

```rust
use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq)]
pub enum Value<'a> {
    Null,
    Bool(bool),
    Number(f64),
    String(Cow<'a, str>),
    Array(Vec<Value<'a>>),
    Object(Vec<(Cow<'a, str>, Value<'a>)>),
}

mod json {
    use std::borrow::Cow;

    prpr::grammar! {
        use super::Value;

        pub value: Value<'i>
            = object
            / array
            / s:string { Value::String(s) }
            / n:number { Value::Number(n) }
            / "true" { Value::Bool(true) }
            / "false" { Value::Bool(false) }
            / "null" { Value::Null }

        object: Value<'i> = "{" kvs:(member, ",")* "}" { Value::Object(kvs) }

        member: (Cow<'i, str>, Value<'i>) = k:string ":" v:value { (k, v) }

        array: Value<'i> = "[" es:(value, ",")* "]" { Value::Array(es) }

        atom string: Cow<'i, str> = '"' s:chunk* '"' { unescape(#s) }

        atom chunk = [^ '"' '\\' '\0'-'\x1F']+ / "\\" (['"' '\\' '/' b f n r t] / "u" hex hex hex hex)

        atom hex = [0-9a-fA-F]
#
#         atom number: f64 = s:("-"? ("0" / [1-9] [0-9]*) ("." [0-9]+)? ([eE] [+-]? [0-9]+)?) {
#             #s.parse().unwrap()
#         }
    }

    // Decodes the escapes of a string literal that the grammar has checked.
    fn unescape(s: &str) -> Cow<'_, str> {
        if !s.contains('\\') {
            return Cow::Borrowed(s);
        }
        let mut out = String::new();
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            out.push(match chars.next().unwrap() {
                'b' => '\x08',
                'f' => '\x0c',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    char::from_u32(u32::from_str_radix(&hex, 16).unwrap()).unwrap_or('\u{fffd}')
                }
                c => c,
            });
        }
        Cow::Owned(out)
    }
}

fn main() {
    let input = r#"{"plain": "text", "escaped": "a\nb"}"#;
    let Ok(Value::Object(members)) = json::value(input) else { panic!() };
    assert!(matches!(&members[0].1, Value::String(Cow::Borrowed("text"))));
    assert!(matches!(&members[1].1, Value::String(Cow::Owned(s)) if s == "a\nb"));
}
```

Now `string` only recognizes the string, and `#s` is the text between the quotes, a `&'i str` borrowed from the input.
The `unescape` function borrows it unless there is an escape to decode.
`chunk` matches a run of plain characters at once, which is also faster than matching them one by one.

This is close to the JSON parser in prpr's [benchmarks](https://github.com/tanakh/prpr/tree/master/benchmarks).
