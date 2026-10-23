// Mistakes in expressions.

mod ranges {
    prpr::grammar! {
        a = [z-a]
        //~^ ERROR: the range `z-a` is reversed
    }
}

mod paths {
    prpr::grammar! {
        a = ::b
        //~^ ERROR: a reference to a rule of another grammar needs the path of its module
    }
}

mod bindings {
    prpr::grammar! {
        a: (char, char) = x:[a-z] x:[0-9] { (x, x) }
        //~^ ERROR: `x` is bound more than once in this sequence
    }
}

mod byte_literals {
    prpr::grammar! {
        a = b"x"
        //~^ ERROR: byte literals require `#![input = bytes]`
    }
}

mod byte_classes {
    prpr::grammar! {
        #![input = bytes]
        a = [a-'\u{100}']
        //~^ ERROR: a character class for byte input can only contain characters up to
    }
}

mod self_paths {
    prpr::grammar! {
        a = self::b
        //~^ ERROR: a rule of this grammar is referred to by its name, without `self::`
        b = "x"
    }
}
