prpr::grammar! {
    a = ("x"?)*
    //~^ ERROR: repetition of an expression that can match empty input
}

fn main() {}
