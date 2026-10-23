// Semantic predicates.

prpr::grammar! {
    atom decimal: i64 = "0" { 0 } / s:([1-9] [0-9]*) { #s.parse().unwrap() }

    mul_of_seven: i64 = d:decimal &{ if d % 7 == 0 { Some(d) } else { None } }

    even: i64 = d:decimal &{ if d % 2 == 0 { Ok(d) } else { Err("an even number") } }

    small = d:decimal &{ d < 10 }

    #[label = "number"]
    labeled_even: i64 = d:decimal &{ if d % 2 == 0 { Ok(d) } else { Err("an even number") } }
}

#[test]
fn option() {
    assert_eq!(mul_of_seven("7"), Ok(7));
    assert_eq!(mul_of_seven("14"), Ok(14));
    assert!(mul_of_seven("").is_err());
    let e = mul_of_seven("100").unwrap_err();
    assert_eq!(e.to_string(), "unexpected `1`");
    assert_eq!(e.offset(), 0);
}

#[test]
fn result() {
    assert_eq!(even("42"), Ok(42));
    let e = even("  41").unwrap_err();
    assert_eq!(e.to_string(), "expected an even number, found `4`");
    assert_eq!(e.offset(), 2);
    // A label does not hide the message.
    let e = labeled_even("41").unwrap_err();
    assert_eq!(e.to_string(), "expected an even number, found `4`");
    assert_eq!(
        labeled_even("x").unwrap_err().to_string(),
        "expected number, found `x`"
    );
}

#[test]
fn bool() {
    assert_eq!(small("9"), Ok(()));
    assert!(small("10").is_err());
}
