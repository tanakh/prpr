prpr::grammar! {
    a = "x" / / "y"
    //~^ ERROR: syntax error: expected expression
}

prpr::grammar! {
    b = "x" @ "y"
    //~^ ERROR: syntax error
}

fn main() {}
