// Grammars in files, with `include_grammar!`.

mod list {
    prpr::include_grammar!("grammars/list.peg");
}

#[test]
fn include_grammar() {
    assert_eq!(list::list("[1, 2, 3]"), Ok(vec![1, 2, 3]));
    assert_eq!(
        list::list("[1, ]").unwrap_err().to_string(),
        "expected number, found `]`"
    );
}

#[test]
fn include_grammar_in_a_function() {
    prpr::include_grammar!(r"grammars/list.peg");

    assert_eq!(list("[4]"), Ok(vec![4]));
}
