use crate::{Value, unescape};
use nom::branch::alt;
use nom::bytes::complete::{tag, take_while_m_n, take_while1};
use nom::character::complete::{char, multispace0, one_of};
use nom::combinator::{map, opt, recognize, value};
use nom::multi::{many0, separated_list0};
use nom::sequence::{delimited, preceded, separated_pair};
use nom::{IResult, Parser};

fn ws<'a, O>(
    p: impl Parser<&'a str, Output = O, Error = nom::error::Error<&'a str>>,
) -> impl Parser<&'a str, Output = O, Error = nom::error::Error<&'a str>> {
    delimited(multispace0, p, multispace0)
}

fn string(i: &str) -> IResult<&str, String> {
    let chunk = alt((
        take_while1(|c: char| c != '"' && c != '\\' && c >= ' '),
        recognize(preceded(
            char('\\'),
            alt((
                recognize(one_of("\"\\/bfnrt")),
                recognize(preceded(
                    char('u'),
                    take_while_m_n(4, 4, |c: char| c.is_ascii_hexdigit()),
                )),
            )),
        )),
    ));
    map(
        delimited(char('"'), recognize(many0(chunk)), char('"')),
        unescape,
    )
    .parse(i)
}

fn number(i: &str) -> IResult<&str, f64> {
    let digits = || take_while1(|c: char| c.is_ascii_digit());
    map(
        recognize((
            opt(char('-')),
            alt((tag("0"), recognize((one_of("123456789"), opt(digits()))))),
            opt((char('.'), digits())),
            opt((one_of("eE"), opt(one_of("+-")), digits())),
        )),
        |s: &str| s.parse().unwrap(),
    )
    .parse(i)
}

fn json_value(i: &str) -> IResult<&str, Value> {
    alt((
        map(
            delimited(
                char('{'),
                separated_list0(
                    ws(char(',')),
                    separated_pair(ws(string), char(':'), ws(json_value)),
                ),
                ws(char('}')),
            ),
            Value::Object,
        ),
        map(
            delimited(
                char('['),
                separated_list0(char(','), ws(json_value)),
                ws(char(']')),
            ),
            Value::Array,
        ),
        map(string, Value::String),
        map(number, Value::Number),
        value(Value::Bool(true), tag("true")),
        value(Value::Bool(false), tag("false")),
        value(Value::Null, tag("null")),
    ))
    .parse(i)
}

pub fn parse(s: &str) -> Value {
    let (rest, v) = ws(json_value).parse(s).unwrap();
    assert!(rest.is_empty());
    v
}
