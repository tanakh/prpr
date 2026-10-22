prpr::grammar! {
    pair: (String, String) = a:word "," b:word { (#a.to_string(), #c.to_string()) }
    //~^ ERROR: `#c` does not refer to a binding of this sequence
    atom word = [a-z]+
}

fn main() {}
