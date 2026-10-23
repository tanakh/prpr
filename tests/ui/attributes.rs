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

mod grammar_attributes {
    prpr::grammar! {
        #![lint(memo)]
        //~^ ERROR: expected `#![lint]` without arguments
        a = "x"
    }
}

fn main() {}
