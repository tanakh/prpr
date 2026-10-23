prpr::grammar! {
    expr = term ("+" term)*
    term = factor
    //~^ ERROR: cannot find rule `factor`
    fact = "x"
}
