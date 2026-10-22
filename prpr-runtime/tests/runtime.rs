use prpr_runtime::*;

#[test]
fn memo_table() {
    let mut table = MemoTable::<String>::default();
    assert_eq!(table.get(0, String::clone), None);
    for pos in (0..1000).step_by(3) {
        let result = if pos % 2 == 0 {
            Ok((pos.to_string(), pos + 1))
        } else {
            Err(Fail)
        };
        table.insert(pos, &result, String::clone);
    }
    for pos in 0..1000 {
        let expected = match pos {
            _ if pos % 3 != 0 => None,
            _ if pos % 2 == 0 => Some(Ok((pos.to_string(), pos + 1))),
            _ => Some(Err(Fail)),
        };
        assert_eq!(table.get(pos, String::clone), expected);
    }
    table.insert(3, &Ok(("x".to_string(), 4)), String::clone);
    assert_eq!(table.get(3, String::clone), Some(Ok(("x".to_string(), 4))));
}

#[test]
fn errors() {
    let found = |s: &str| Found::Char(s.chars().next().unwrap());
    let e = ParseError::new(0, vec![], found("x"));
    assert_eq!(e.to_string(), "unexpected `x`");
    let e = ParseError::new(0, vec![Expected::Literal("\"\n")], Found::Eof);
    assert_eq!(e.to_string(), "expected `\"\\n`, found end of input");
    let e = ParseError::new(
        0,
        vec![Expected::Literal("a"), Expected::Label("number")],
        found("?"),
    );
    assert_eq!(e.to_string(), "expected one of `a` or number, found `?`");
    let e = ParseError::new(
        0,
        vec![Expected::Class("[a-z]"), Expected::Any, Expected::Eof],
        Found::Byte(0xff),
    );
    assert_eq!(
        e.to_string(),
        "expected one of `[a-z]`, any character, or end of input, found `b'\\xff'`"
    );
}

#[test]
fn render() {
    let src = "first\n\tsecond line\n";
    let e = ParseError::new(7, vec![Expected::Literal("x")], Found::Char('s'));
    assert_eq!(e.line_col(src), LineCol { line: 2, column: 2 });
    assert_eq!(
        e.render(src, "file.txt").to_string(),
        "\
error: expected `x`, found `s`
 --> file.txt:2:2
  |
2 | \tsecond line
  | \t^"
    );
    let at_end = ParseError::new(src.len(), vec![], Found::Eof);
    assert_eq!(at_end.line_col(src), LineCol { line: 3, column: 1 });
}

#[test]
fn spans() {
    assert_eq!(Span::new(3, 1), Span::new(1, 1));
    assert!(Span::new(1, 5).contains(Span::new(2, 5)));
    assert!(!Span::new(1, 5).contains(Span::new(0, 2)));
    assert_eq!(Span::from(2..4).len(), 2);
}
