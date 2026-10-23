prpr::grammar! {
    a = "x"
    match = "y"
    //~^ ERROR: `match` is a keyword and cannot be a rule name
}
