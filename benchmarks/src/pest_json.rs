use crate::{Value, unescape};
use pest::Parser;
use pest::iterators::Pair;

#[derive(pest_derive::Parser)]
#[grammar = "json.pest"]
struct JsonParser;

fn value(pair: Pair<'_, Rule>) -> Value {
    match pair.as_rule() {
        Rule::object => Value::Object(
            pair.into_inner()
                .map(|pair| {
                    let mut inner = pair.into_inner();
                    let key = string(inner.next().unwrap());
                    (key, value(inner.next().unwrap()))
                })
                .collect(),
        ),
        Rule::array => Value::Array(pair.into_inner().map(value).collect()),
        Rule::string => Value::String(string(pair)),
        Rule::number => Value::Number(pair.as_str().parse().unwrap()),
        Rule::boolean => Value::Bool(pair.as_str() == "true"),
        Rule::null => Value::Null,
        _ => unreachable!(),
    }
}

fn string(pair: Pair<'_, Rule>) -> String {
    unescape(pair.into_inner().next().unwrap().as_str())
}

pub fn parse(s: &str) -> Value {
    value(JsonParser::parse(Rule::json, s).unwrap().next().unwrap())
}
