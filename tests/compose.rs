// Rules of one grammar used from another.

mod lexical {
    prpr::grammar! {
        pub atom ident: &'i str = s:([a-z]+) { #s }
        pub atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
        pub list: Vec<i64> = "[" xs:(number, ",")* "]" { xs }
        #[boundary(![a-z0-9.])]
        pub atom version: &'i str = s:([0-9]+ "." [0-9]+) { #s }
    }
}

mod config {
    prpr::grammar! {
        pub config: Vec<(&'i str, Value<'i>)> = entry*

        entry: (&'i str, Value<'i>) = k:super::lexical::ident "=" v:value ";" { (k, v) }

        value: Value<'i>
            = n:super::lexical::number { Value::Number(n) }
            / l:super::lexical::list { Value::List(l) }
            / "v" v:super::lexical::version { Value::Version(v) }
    }

    #[derive(Debug, PartialEq)]
    pub enum Value<'a> {
        Number(i64),
        List(Vec<i64>),
        Version(&'a str),
    }
}

use config::Value;

#[test]
fn compose() {
    assert_eq!(
        config::config("a = 1; bb = [1, 2] ;c=v 1.2;"),
        Ok(vec![
            ("a", Value::Number(1)),
            ("bb", Value::List(vec![1, 2])),
            ("c", Value::Version("1.2"))
        ])
    );
}

#[test]
fn tokens_of_other_grammars_are_delimited() {
    // The default boundary of `number`.
    let e = config::config("a = 1x;").unwrap_err();
    assert_eq!(
        (e.to_string(), e.offset()),
        ("expected delimiter, found `x`".into(), 5)
    );
    // The custom boundary of `version`.
    let e = config::config("a = v 1.2.3;").unwrap_err();
    assert_eq!(
        (e.to_string(), e.offset()),
        ("expected delimiter, found `.`".into(), 9)
    );
}

#[test]
fn errors_inside_other_grammars() {
    let e = config::config("a = [1, ];").unwrap_err();
    assert_eq!(
        (e.to_string(), e.offset()),
        ("expected `[0-9]`, found `]`".into(), 8)
    );
}
