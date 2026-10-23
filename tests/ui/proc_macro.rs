// A proc-macro defined with `prpr::define_proc_macro!` reports errors at the tokens of its input.

use calc::calc;

pub fn f(width: i64) -> i64 {
    // A parse error, at the token where the parser failed.
    let a = calc!(width + * 2);
    //~^ ERROR: expected one of `(`, number, or variable, found `*`
    // A `MacroError` of the expansion, at the span it is about.
    let b = calc!(width / 0);
    //~^ ERROR: division by zero
    // An error of the compiler about a token that the expansion passed on.
    let c = calc!(width * hieght);
    //~^ ERROR: cannot find value `hieght` in this scope
    a + b + c
}
