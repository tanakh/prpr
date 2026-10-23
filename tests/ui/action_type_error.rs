//@no-rustfix
prpr::grammar! {
    num: i64 = s:([0-9]+) { #s.len() }
    //~^ ERROR: mismatched types
}
