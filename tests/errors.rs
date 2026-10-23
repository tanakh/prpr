// Error messages.

prpr::grammar! {
    test_choice = "foo" / "bar" / "baz"
    test_seq = "foo" "bar"
    test_any = "foo" .
    test_eof = "foo" !.
    test_class = [a-zA-Z0-9_]
    test_classes = [a-zA-Z0-9_]* !.

    test_label = test_decimal
    #[label = "decimal"]
    atom test_decimal = [1-9] [0-9]*

    test_list = "[" (item, ",")* "]"
    #[label = "item"]
    item = "<" [a-z]+ ">"

    test_keyword = "let" ident
    #[label = "identifier"]
    atom ident = !keyword [a-z]+
    atom keyword = "let" ![a-z]
}

fn err(r: Result<(), prpr::ParseError>) -> (String, usize) {
    let e = r.unwrap_err();
    (e.to_string(), e.offset())
}

#[test]
fn messages() {
    assert_eq!(
        err(test_choice("hoge")),
        (
            "expected one of `bar`, `baz`, or `foo`, found `h`".into(),
            0
        )
    );
    assert_eq!(
        err(test_seq("foo baz")),
        ("expected `bar`, found `b`".into(), 4)
    );
    assert_eq!(
        err(test_any("foo")),
        ("expected any character, found end of input".into(), 3)
    );
    assert_eq!(
        err(test_eof("foo bar")),
        ("expected end of input, found `b`".into(), 4)
    );
    assert_eq!(
        err(test_class("?")),
        ("expected `[a-zA-Z0-9_]`, found `?`".into(), 0)
    );
    assert_eq!(
        err(test_class("")),
        ("expected `[a-zA-Z0-9_]`, found end of input".into(), 0)
    );
    assert_eq!(
        err(test_classes("hoge;")),
        (
            "expected one of `[a-zA-Z0-9_]` or end of input, found `;`".into(),
            4
        )
    );
}

#[test]
fn labels() {
    assert_eq!(
        err(test_label("")),
        ("expected decimal, found end of input".into(), 0)
    );
    assert_eq!(
        err(test_label("?")),
        ("expected decimal, found `?`".into(), 0)
    );
    assert_eq!(
        err(test_label("  012")),
        ("expected decimal, found `0`".into(), 2)
    );
    assert_eq!(
        err(test_label("1!")),
        ("expected end of input, found `!`".into(), 1)
    );

    // A label replaces what is expected at the start of the rule only.
    assert_eq!(
        err(test_list("[<a>, ]")),
        ("expected item, found `]`".into(), 6)
    );
    assert_eq!(
        err(test_list("[<a>, <b]")),
        ("expected one of `>` or `[a-z]`, found `]`".into(), 8)
    );

    // A label also replaces a failure that expected nothing in particular.
    assert_eq!(
        err(test_keyword("let let")),
        ("expected identifier, found `l`".into(), 4)
    );
}

#[test]
fn render() {
    let src = "foo\nfoo baz";
    let e = test_seq(src).unwrap_err();
    assert_eq!(
        e.render(src, "<input>").to_string(),
        "\
error: expected `bar`, found `f`
 --> <input>:2:1
  |
2 | foo baz
  | ^"
    );
}

mod comments {
    prpr::grammar! {
        pub words = word*
        atom word = [a-z]+
        atom comment = "//" (!"\n" .)* / "/*" (!"*/" .)* "*/"
    }
}

#[test]
fn comments() {
    // A comment that started and cannot end is reported where its end was expected.
    assert_eq!(
        err(comments::words("a /* b")),
        (
            "expected one of `*/` or any character, found end of input".into(),
            6
        )
    );
    // Otherwise, comments are never expected.
    assert_eq!(
        err(comments::words("a /* b */ 1")),
        (
            "expected one of `[a-z]` or end of input, found `1`".into(),
            10
        )
    );
}
