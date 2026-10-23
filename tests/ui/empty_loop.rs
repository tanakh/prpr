prpr::grammar! {
    a = ("x"?)*
    //~^ ERROR: repetition of an expression that can match empty input
}

mod boundaries {
    prpr::grammar! {
        #[boundary((!"x")*)]
        //~^ ERROR: repetition of an expression that can match empty input
        atom number = [0-9]+
    }
}
