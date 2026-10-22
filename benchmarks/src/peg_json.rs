use crate::{Value, unescape};

peg::parser! {
    grammar json() for str {
        pub rule document() -> Value = _ v:value() _ { v }

        rule value() -> Value
            = object()
            / array()
            / s:string() { Value::String(s) }
            / n:number() { Value::Number(n) }
            / "true" { Value::Bool(true) }
            / "false" { Value::Bool(false) }
            / "null" { Value::Null }

        rule object() -> Value
            = "{" _ kvs:(member() ** (_ "," _)) _ "}" { Value::Object(kvs) }

        rule member() -> (String, Value) = k:string() _ ":" _ v:value() { (k, v) }

        rule array() -> Value
            = "[" _ es:(value() ** (_ "," _)) _ "]" { Value::Array(es) }

        rule string() -> String = "\"" s:$(chunk()*) "\"" { unescape(s) }

        rule chunk()
            = [^ '"' | '\\' | '\0'..='\x1F']+
            / "\\" (['"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't'] / "u" hex() hex() hex() hex())

        rule hex() = ['0'..='9' | 'a'..='f' | 'A'..='F']

        rule number() -> f64
            = s:$("-"? ("0" / ['1'..='9'] ['0'..='9']*) ("." ['0'..='9']+)? (['e' | 'E'] ['+' | '-']? ['0'..='9']+)?)
            { s.parse().unwrap() }

        rule _() = quiet!{[' ' | '\t' | '\r' | '\n']*}
    }
}

pub fn parse(s: &str) -> Value {
    json::document(s).unwrap()
}
