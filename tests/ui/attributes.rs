prpr::grammar! {
    #[memoize]
    //~^ ERROR: unknown attribute `memoize`
    a = "x"

    #[boundary(!"y")]
    //~^ ERROR: `#[boundary]` and `#[no_boundary]` only apply to `atom` rules
    b = "x"

    comment = "//" (!"\n" .)*
    //~^ ERROR: `comment` must be an `atom` rule

    #[memo]
    #[no_memo]
    c = "x"
    //~^ ERROR: a rule cannot have both `#[memo]` and `#[no_memo]`
}

mod arguments {
    prpr::grammar! {
        #[memo = 1]
        //~^ ERROR: expected `#[memo]` without arguments
        a = "x"

        #[memo(a)]
        //~^ ERROR: expected `#[memo]` without arguments
        b = "x"

        #[label]
        //~^ ERROR: expected `#[label = "..."]`
        c = "x"

        #[label = some words]
        //~^ ERROR: expected `#[label = "..."]`
        d = "x"

        #[boundary(1)]
        //~^ ERROR: expected `#[boundary(expression)]`
        atom e = "x"
    }
}

mod grammar_attributes {
    prpr::grammar! {
        #![lint(memo)]
        //~^ ERROR: expected `#![lint]` without arguments
        #![input = string]
        //~^ ERROR: expected `#![input = str]` or `#![input = bytes]`
        #![inputs = str]
        //~^ ERROR: unknown grammar attribute `inputs`
        a = "x"
    }
}
