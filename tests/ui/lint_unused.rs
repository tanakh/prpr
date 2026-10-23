// With `#![lint]`, private rules that neither rules nor Rust code use are errors.

pub mod unused {
    prpr::grammar! {
        #![lint]
        pub list = "[" (item, ",")* "]"
        item = "x"
        unused = "y" unused?
        //~^ ERROR: function `unused` is never used
        used_by_rust = "z"
    }

    pub fn f() -> bool {
        used_by_rust("z").is_ok()
    }
}

// Rules that implicit whitespace and delimiter checks use are used.
pub mod implicit {
    prpr::grammar! {
        #![lint]
        pub list = "[" ("x", ",")* "]"
        atom spaces = [' ' '\n']*
        atom word_char = [a-z]
    }
}
