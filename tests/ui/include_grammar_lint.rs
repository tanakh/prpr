// With `#![lint]`, unused rules of included grammars are reported at the path.

pub mod included {
    prpr::include_grammar!("include_grammar_lint.peg");
    //~^ ERROR: function `c` is never used
}
