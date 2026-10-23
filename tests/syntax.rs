// Every construct of the grammar language.

prpr::grammar! {
    p_any: char = .
    p_ref: char = p_any
    p_lit = "abc"
    p_char = 'a'
    p_class: char = [a-z]
    p_neg_class: char = [^a-z]
    p_seq = "foo" "bar"
    p_choice: &'static str = "foo" / "fff"
    atom p_many: usize = s:"a"* { s.len() }
    atom p_many1: usize = s:"a"+ { s.len() }
    p_opt: bool = s:"foo"? { s.is_some() }
    p_and = &"foo" "foo" "bar"
    p_not = !"foo" .
    p_sep: Vec<char> = ([a-z], ",")*
    p_sep1: Vec<char> = ([a-z], ",")+
    p_atomic = <"foo" "bar">
    p_action: u8 = c:[0-9] { c as u8 - b'0' }
    p_empty_action: i32 = { 42 }
    p_keyword: u8 = s:([a-z]+) { if #s == "a" { 1 } else { match #s { "b" => 2, _ => 3 } } }
    p_format: String = c:[a-z] { format!("<{c}>") }
    p_group: char = c:[a-z] (c:[0-9]) { c }
    p_tuple: (char, char) = "(" [a-z] "," [a-z] ")"
    p_nested: Vec<(char, Option<char>)> = ([a-z] [0-9]?)*

    atomic_test1 = "abc" <"def" "ghi"> "jkl"
    atomic_test2 = "+" <"-" "*"> "/"
}

#[test]
fn terminals() {
    assert_eq!(p_any("a"), Ok('a'));
    // `.` is a token, so whitespace before it is skipped.
    assert!(p_any(" ").is_err());
    assert_eq!(p_any(" a "), Ok('a'));
    assert_eq!(p_any("猫"), Ok('猫'));
    assert!(p_any("").is_err());
    assert_eq!(p_ref(" x "), Ok('x'));

    assert_eq!(p_lit("abc"), Ok(()));
    assert_eq!(p_lit("   abc  "), Ok(()));
    assert!(p_lit("abcd").is_err());
    assert!(p_lit("ab").is_err());
    assert_eq!(p_char("a"), Ok(()));

    assert_eq!(p_class("a"), Ok('a'));
    assert!(p_class("A").is_err());
    assert!(p_class("").is_err());
    assert_eq!(p_neg_class("A"), Ok('A'));
    assert_eq!(p_neg_class("あ"), Ok('あ'));
    assert!(p_neg_class("a").is_err());
}

#[test]
fn sequences_and_choices() {
    assert_eq!(p_seq("foo bar"), Ok(()));
    assert_eq!(p_seq("   foo   bar"), Ok(()));
    // Tokens must be delimited.
    assert!(p_seq("foobar").is_err());

    assert_eq!(p_choice("foo"), Ok("foo"));
    assert_eq!(p_choice("fff"), Ok("fff"));
    assert!(p_choice("ff").is_err());
}

#[test]
fn repetitions() {
    assert_eq!(p_many(""), Ok(0));
    assert_eq!(p_many("aaa"), Ok(3));
    assert!(p_many("aaab").is_err());
    assert!(p_many1("").is_err());
    assert_eq!(p_many1("a"), Ok(1));
    assert_eq!(p_many1("aa"), Ok(2));

    assert_eq!(p_opt("foo"), Ok(true));
    assert_eq!(p_opt(""), Ok(false));

    assert_eq!(p_sep(""), Ok(vec![]));
    assert_eq!(p_sep("a"), Ok(vec!['a']));
    assert_eq!(p_sep("a,b"), Ok(vec!['a', 'b']));
    assert_eq!(p_sep("a , b"), Ok(vec!['a', 'b']));
    assert!(p_sep("a,b,").is_err());
    assert!(p_sep1("").is_err());
    assert_eq!(p_sep1("a, b, c"), Ok(vec!['a', 'b', 'c']));

    assert_eq!(
        p_nested("a1 b c3"),
        Ok(vec![('a', Some('1')), ('b', None), ('c', Some('3'))])
    );
}

#[test]
fn lookahead() {
    assert_eq!(p_and("foo bar"), Ok(()));
    assert!(p_and("bar").is_err());
    assert_eq!(p_not("b"), Ok(()));
    assert!(p_not("bar").is_err());
    assert!(p_not("foo").is_err());
}

#[test]
fn atomic() {
    assert_eq!(p_atomic("foobar"), Ok(()));
    assert_eq!(p_atomic("  foobar"), Ok(()));
    assert!(p_atomic("foo bar").is_err());

    assert_eq!(atomic_test1("abc defghi jkl"), Ok(()));
    assert!(atomic_test1("abcdefghijkl").is_err());
    assert!(atomic_test1("abc def ghi jkl").is_err());

    assert_eq!(atomic_test2("+-*/"), Ok(()));
    assert_eq!(atomic_test2("+ -* /"), Ok(()));
    assert!(atomic_test2("+- */").is_err());
}

#[test]
fn actions() {
    assert_eq!(p_action("7"), Ok(7));
    assert_eq!(p_empty_action(""), Ok(42));
    assert_eq!(p_empty_action("  "), Ok(42));
    assert_eq!(p_tuple("(a, b)"), Ok(('a', 'b')));
    // `#x` and `@x` after keywords.
    assert_eq!(p_keyword("a"), Ok(1));
    assert_eq!(p_keyword("b"), Ok(2));
    assert_eq!(p_keyword("c"), Ok(3));
    // A binding used by a format string.
    assert_eq!(p_format("x"), Ok("<x>".into()));
    // A binding in a group is local to it.
    assert_eq!(p_group("a1"), Ok('a'));
}
