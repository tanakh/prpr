// The checks of `#![lint]`.

mod memo {
    prpr::grammar! {
        #![lint]
        // `term` is parsed again after the common start.
        pub term: i64
            = "(" t:term ")" "!" { t + 1 }
            / "(" t:term ")" { t }
            //~^ ERROR: `term` may be parsed more than once
            / "x" { 0 }

        // Both alternatives start with `postfix`, through other rules.
        pub expr = lhs "=" expr / sum
        //~^ ERROR: `postfix` may be parsed more than once
        lhs = postfix
        sum = postfix ("+" postfix)*
        postfix = name ("." name)*
        atom name = [a-z]+

        // Not reported: the rule is memoized, or not memoized on purpose.
        #[memo]
        pub memoized = "(" memoized ")" "!" / "(" memoized ")" / "x"
        #[no_memo]
        pub intended = "(" intended ")" "!" / "(" intended ")" / "x"
    }
}

mod unreachable {
    prpr::grammar! {
        #![lint]
        pub a = "x"? / "y"
        //~^ ERROR: this alternative is never tried
        // Not reported: the alternative before the last one can fail.
        pub b = "x" / { () }
    }
}

mod shadowed {
    prpr::grammar! {
        #![lint]
        pub op = "<" / "<="
        //~^ ERROR: `"<="` never matches here
        pub num: i64 = n:int { n } / n:int "." { n }
        //~^ ERROR: this alternative never matches
        // Not reported: `if` is not delimited in `ifdef`, and `x` must follow `<`.
        pub kw = "if" / "ifdef"
        pub ok = "<" x / "<=" x
        atom x = "x"
        atom int: i64 = s:([0-9]+) { #s.parse().unwrap() }
    }
}
