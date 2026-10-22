// Implicit whitespace and delimiters.

mod default {
    prpr::grammar! {
        pub call = ident "(" (number, ",")* ")"
        pub keyword = "if" ident
        atom ident = [a-z]+
        atom number = [0-9]+
    }
}

#[test]
fn delimiters() {
    assert!(default::call("f(1, 2)").is_ok());
    assert!(default::call("f ( 1 , 2 ) ").is_ok());
    let e = default::call("f(1x)").unwrap_err();
    assert_eq!(
        (e.to_string(), e.offset()),
        ("expected delimiter, found `x`".into(), 3)
    );

    assert!(default::keyword("if x").is_ok());
    // `"if"` does not match the start of `ifx`.
    assert!(default::keyword("ifx").is_err());
}

mod custom_boundary {
    prpr::grammar! {
        pub numbers: Vec<&'i str> = number*
        #[boundary(![a-z0-9.])]
        atom number: &'i str = s:([0-9]+) { #s }
        pub words: Vec<&'i str> = word*
        #[no_boundary]
        atom word: &'i str = s:([a-z]+) { #s }
    }
}

#[test]
fn custom_boundaries() {
    use custom_boundary::*;
    assert_eq!(numbers("1 2 3"), Ok(vec!["1", "2", "3"]));
    let e = numbers("1 2.x").unwrap_err();
    assert_eq!(
        (e.to_string(), e.offset()),
        ("expected delimiter, found `.`".into(), 3)
    );
    // Without a boundary check, longest matches follow each other.
    assert_eq!(words("abc def"), Ok(vec!["abc", "def"]));
}

mod comments {
    prpr::grammar! {
        pub list: Vec<&'i str> = word*
        atom word: &'i str = s:([a-z]+) { #s }
        atom comment = "//" (!"\n" .)* / "/*" (!"*/" .)* "*/"
    }
}

#[test]
fn custom_comments() {
    assert_eq!(
        comments::list("a // b\n c /* d */ e"),
        Ok(vec!["a", "c", "e"])
    );
}

mod no_newlines {
    prpr::grammar! {
        pub lines: Vec<Vec<&'i str>> = (line, "\n")*
        line: Vec<&'i str> = word*
        atom word: &'i str = s:([a-z]+) { #s }
        atom whitespace = [' ' '\t']
    }
}

#[test]
fn custom_whitespace() {
    assert_eq!(
        no_newlines::lines("a b\n c"),
        Ok(vec![vec!["a", "b"], vec!["c"]])
    );
}

mod words {
    prpr::grammar! {
        pub kw = "let" name
        atom name = [a-z]+
        atom word_char = [a-z0-9_-]
    }
}

#[test]
fn custom_word_chars() {
    assert!(words::kw("let x").is_ok());
    assert!(words::kw("let-x").is_err());
}

mod interpolation {
    prpr::grammar! {
        // A lexical rule may call a syntactic rule, which skips whitespace before its tokens.
        // Whitespace after it is up to the lexical rule.
        pub atom string: Vec<&'i str> = '"' parts:part* '"' { parts }
        atom part: &'i str = "${" e:expr spaces "}" { e } / s:([a-z ' ']+) { #s }
        expr: &'i str = s:ident { s }
        atom ident: &'i str = s:([a-z]+) { #s }
    }
}

#[test]
fn lexical_calls_syntactic() {
    assert_eq!(
        interpolation::string(r#""a ${ x } b""#),
        Ok(vec!["a ", "x", " b"])
    );
}
