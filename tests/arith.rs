mod arith {
    prpr::grammar! {
        pub arith: i64 = expr

        expr: i64 = l:term rs:(("+" / "-") term)* {
            rs.into_iter().fold(l, |a, (op, b)| if op == "+" { a + b } else { a - b })
        }

        term: i64 = l:fact rs:(("*" / "/") fact)* {
            rs.into_iter().fold(l, |a, (op, b)| if op == "*" { a * b } else { a / b })
        }

        fact: i64 = decimal / "(" expr ")"

        atom decimal: i64 = "0" { 0 } / s:([1-9] [0-9]*) { #s.parse().unwrap() }
    }
}

#[test]
fn arith() {
    assert_eq!(arith::arith("0"), Ok(0));
    assert_eq!(arith::arith("123"), Ok(123));
    assert_eq!(arith::arith("123 + 456"), Ok(579));
    assert_eq!(arith::arith("1 + 2 * 3"), Ok(7));
    assert_eq!(arith::arith(" (1 + 2) * 3 "), Ok(9));
    assert_eq!(arith::arith("(((((0)))))"), Ok(0));
    assert_eq!(arith::arith("10 - 2 - 3"), Ok(5));
}

#[test]
fn arith_errors() {
    let e = arith::arith("012").unwrap_err();
    assert_eq!(e.to_string(), "expected delimiter, found `1`");
    assert_eq!(e.offset(), 1);
    let e = arith::arith("1 +").unwrap_err();
    assert_eq!(
        e.to_string(),
        "expected one of `(`, `0`, or `[1-9]`, found end of input"
    );
    let e = arith::arith("(1 + 2").unwrap_err();
    assert_eq!(e.offset(), 6);
}
