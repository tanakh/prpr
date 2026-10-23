// Mistakes in references to the rules of other grammars, each reported once, at the reference.

mod lexical {
    prpr::grammar! {
        pub atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
        atom word = [a-z]+
    }
}

mod binary {
    prpr::grammar! {
        #![input = bytes]
        pub atom byte = .
    }
}

mod uses {
    prpr::grammar! {
        pub misspelled: i64 = n:super::lexical::nmber { n }
        //~^ ERROR: no variant, associated function, or constant named `nmber`
        pub private = super::lexical::word
        //~^ ERROR: associated constant `word` is private
        pub other_input = super::binary::byte
        //~^ ERROR: this rule of another grammar parses another kind of input than this grammar
    }
}

fn main() {}
