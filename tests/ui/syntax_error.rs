prpr::grammar! {
    a = "x" / / "y"
    //~^ ERROR: syntax error: expected expression
}

prpr::grammar! {
    b = [z-a]
    //~^ ERROR: range in ascending order
}

fn main() {}
