prpr::grammar! {
    a = "x"
    a = "y"
    //~^ ERROR: rule `a` is defined more than once
}

fn main() {}
