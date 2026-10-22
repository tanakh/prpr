// The example of README.md.

prpr::grammar! {
    pub expr: i64 = l:term rs:(("+" / "-") term)* {
        rs.into_iter().fold(l, |a, (op, b)| if op == "+" { a + b } else { a - b })
    }
    term: i64 = l:fact rs:(("*" / "/") fact)* {
        rs.into_iter().fold(l, |a, (op, b)| if op == "*" { a * b } else { a / b })
    }
    fact: i64 = number / "(" e:expr ")" { e }

    #[label = "number"]
    atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
}

#[test]
fn readme() {
    assert_eq!(expr("1 + 2 * (3 - 1)"), Ok(5));
    assert_eq!(
        expr("1 +").unwrap_err().to_string(),
        "expected one of `(` or number, found end of input"
    );
}
