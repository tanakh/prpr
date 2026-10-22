prpr::grammar! {
    #[memoize]
    //~^ ERROR: unknown attribute `memoize`
    a = "x"

    #[boundary(!"y")]
    //~^ ERROR: `#[boundary]` and `#[no_boundary]` only apply to `atom` rules
    b = "x"

    comment = "//" (!"\n" .)*
    //~^ ERROR: `comment` must be an `atom` rule
}

fn main() {}
