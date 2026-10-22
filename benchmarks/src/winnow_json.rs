use crate::{Value, unescape};
use winnow::ascii::multispace0;
use winnow::combinator::{alt, delimited, opt, preceded, repeat, separated, separated_pair};
use winnow::prelude::*;
use winnow::token::{one_of, take_while};

type Input<'a> = &'a str;

fn ws<'a, O>(
    p: impl ModalParser<Input<'a>, O, winnow::error::ContextError>,
) -> impl ModalParser<Input<'a>, O, winnow::error::ContextError> {
    delimited(multispace0, p, multispace0)
}

fn string(i: &mut Input<'_>) -> ModalResult<String> {
    let chunk = alt((
        take_while(1.., |c: char| c != '"' && c != '\\' && c >= ' ').void(),
        preceded(
            '\\',
            alt((
                one_of(['"', '\\', '/', 'b', 'f', 'n', 'r', 't']).void(),
                preceded('u', take_while(4, |c: char| c.is_ascii_hexdigit())).void(),
            )),
        ),
    ));
    delimited('"', repeat::<_, _, (), _, _>(0.., chunk).take(), '"')
        .map(unescape)
        .parse_next(i)
}

fn number(i: &mut Input<'_>) -> ModalResult<f64> {
    (
        opt('-'),
        alt((
            "0".void(),
            (
                one_of('1'..='9'),
                take_while(0.., |c: char| c.is_ascii_digit()),
            )
                .void(),
        )),
        opt(('.', take_while(1.., |c: char| c.is_ascii_digit()))),
        opt((
            one_of(['e', 'E']),
            opt(one_of(['+', '-'])),
            take_while(1.., |c: char| c.is_ascii_digit()),
        )),
    )
        .take()
        .map(|s: &str| s.parse().unwrap())
        .parse_next(i)
}

fn json_value(i: &mut Input<'_>) -> ModalResult<Value> {
    alt((
        delimited(
            '{',
            separated(0.., separated_pair(ws(string), ':', ws(json_value)), ','),
            (multispace0, '}'),
        )
        .map(Value::Object),
        delimited('[', separated(0.., ws(json_value), ','), (multispace0, ']')).map(Value::Array),
        string.map(Value::String),
        number.map(Value::Number),
        "true".value(Value::Bool(true)),
        "false".value(Value::Bool(false)),
        "null".value(Value::Null),
    ))
    .parse_next(i)
}

pub fn parse(s: &str) -> Value {
    ws(json_value).parse(s).unwrap()
}
