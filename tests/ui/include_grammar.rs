mod bad {
    prpr::include_grammar!("include_grammar.peg");
    //~^ ERROR: cannot find rule `numbr`
}

mod missing {
    prpr::include_grammar!("no_such_file.peg");
    //~^ ERROR: cannot read
}

mod not_a_literal {
    prpr::include_grammar!(concat!("include_grammar", ".peg"));
    //~^ ERROR: expected the path of a grammar file, as a string literal
}
