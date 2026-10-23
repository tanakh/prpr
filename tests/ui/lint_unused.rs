// With `#![lint]`, private rules that neither rules nor Rust code use are errors.

mod unused {
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

fn main() {
    unused::f();
}
