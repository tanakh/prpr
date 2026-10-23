prpr::grammar! {
    pair: (String, String) = a:word "," b:word { (#a.to_string(), #c.to_string()) }
    //~^ ERROR: `#c` does not refer to a binding of this sequence
    atom word = [a-z]+
}

mod never_run {
    prpr::grammar! {
        // Actions of rules without values never run, but are checked.
        a = x:"a" { #y }
        //~^ ERROR: `#y` does not refer to a binding of this sequence
    }
}
