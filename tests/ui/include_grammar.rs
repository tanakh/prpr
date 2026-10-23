mod bad {
    prpr::include_grammar!("include_grammar.peg");
    //~^ ERROR: cannot find rule `numbr`
}

mod missing {
    prpr::include_grammar!("no_such_file.peg");
    //~^ ERROR: cannot read
}
