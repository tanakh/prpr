use crate::{Value, unescape};

prpr::grammar! {
    pub json: Value
        = object
        / array
        / s:string { Value::String(s) }
        / n:number { Value::Number(n) }
        / "true" { Value::Bool(true) }
        / "false" { Value::Bool(false) }
        / "null" { Value::Null }

    object: Value = "{" kvs:(member, ",")* "}" { Value::Object(kvs) }
    member: (String, Value) = k:string ":" v:json { (k, v) }
    array: Value = "[" es:(json, ",")* "]" { Value::Array(es) }

    atom string: String = '"' s:chunk* '"' { unescape(#s) }
    atom chunk = [^ '"' '\\' '\0'-'\x1F']+ / "\\" (['"' '\\' '/' b f n r t] / "u" hex hex hex hex)
    atom hex = [0-9a-fA-F]

    atom number: f64 = s:("-"? ("0" / [1-9] [0-9]*) ("." [0-9]+)? ([eE] [+-]? [0-9]+)?) {
        #s.parse().unwrap()
    }
}

pub fn parse(s: &str) -> Value {
    json(s).unwrap()
}
