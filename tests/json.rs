// JSON <https://www.json.org/json-en.html>

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Object(Vec<(String, Value)>),
    Array(Vec<Value>),
    String(String),
    Number(f64),
    Bool(bool),
    Null,
}

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

        #[label = "string"]
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

        #[label = "number"]
        atom number: f64 = s:("-"? ("0" / [1-9] [0-9]*) ("." [0-9]+)? ([eE] [+-]? [0-9]+)?) {
            #s.parse().unwrap()
        }
    }
}

use Value::{Array, Bool, Null, Number, Object};

#[allow(non_snake_case)]
fn String(s: std::string::String) -> Value {
    Value::String(s)
}

fn obj(kvs: &[(&str, Value)]) -> Value {
    Object(
        kvs.iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
    )
}

#[test]
fn literals() {
    assert_eq!(json::value("true"), Ok(Bool(true)));
    assert_eq!(json::value("false"), Ok(Bool(false)));
    assert_eq!(json::value(" null "), Ok(Null));
    assert_eq!(json::value(r#""""#), Ok(String("".into())));
    assert_eq!(
        json::value(r#""Hello, World!\n""#),
        Ok(String("Hello, World!\n".into()))
    );
    assert_eq!(
        json::value(r#""\"\\\/\b\f\n\r\t\u732B""#),
        Ok(String("\"\\/\x08\x0c\n\r\t猫".into()))
    );
}

#[test]
fn numbers() {
    for (s, n) in [
        ("0", 0.0),
        ("1", 1.0),
        ("123", 123.0),
        ("2.5", 2.5),
        ("-123", -123.0),
        ("-0", -0.0),
        ("-2.5", -2.5),
        ("1e18", 1e18),
        ("2.5e18", 2.5e18),
        ("2.5E-18", 2.5e-18),
    ] {
        assert_eq!(json::value(s), Ok(Number(n)), "{s}");
    }
    for s in ["0123", ".123", "1.", "1e", "- 1", "01"] {
        assert!(json::value(s).is_err(), "{s}");
    }
}

#[test]
fn compound() {
    assert_eq!(json::value("[]"), Ok(Array(vec![])));
    assert_eq!(json::value("[1]"), Ok(Array(vec![Number(1.0)])));
    assert!(json::value("[1,]").is_err());
    assert_eq!(
        json::value(r#"[1, "Hello", 2.5]"#),
        Ok(Array(vec![
            Number(1.0),
            String("Hello".into()),
            Number(2.5)
        ]))
    );
    assert!(json::value("[[[[[[[[[[]]]]]]]]]]").is_ok());
    assert_eq!(json::value("{}"), Ok(obj(&[])));
    assert_eq!(
        json::value(r#"{"hoge": "moge"}"#),
        Ok(obj(&[("hoge", String("moge".into()))]))
    );
    assert!(json::value(r#"{"hoge": "moge",}"#).is_err());
    assert_eq!(
        json::value(r#"{"foo": "123", "bar": 345, "baz": true}"#),
        Ok(obj(&[
            ("foo", String("123".into())),
            ("bar", Number(345.0)),
            ("baz", Bool(true))
        ]))
    );
    assert_eq!(
        json::value(r#"{"foo": [{"bar": []}]}"#),
        Ok(obj(&[("foo", Array(vec![obj(&[("bar", Array(vec![]))])]))]))
    );
}

#[test]
fn errors() {
    let msg = |s: &str| json::value(s).unwrap_err().to_string();
    assert_eq!(
        msg(""),
        "expected one of `[`, `false`, `null`, `true`, `{`, number, or string, found end of input"
    );
    assert_eq!(
        msg("[1, 2"),
        "expected one of `,` or `]`, found end of input"
    );
    assert_eq!(msg(r#"{"a" 1}"#), "expected `:`, found `1`");
    assert_eq!(
        msg(r#""\x""#),
        "expected one of `\"`, `/`, `\\\\`, `b`, `f`, `n`, `r`, `t`, or `u`, found `x`"
    );
    assert_eq!(
        msg("[nul]"),
        "expected one of `[`, `]`, `false`, `null`, `true`, `{`, number, or string, found `n`"
    );
}
