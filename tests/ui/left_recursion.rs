prpr::grammar! {
    expr: i64 = l:expr "+" r:term { l + r } / term
    //~^ ERROR: left recursion: `expr` -> `expr`
    term: i64 = s:([0-9]+) { #s.parse().unwrap() }
}

mod indirect {
    prpr::grammar! {
        a = b "x"
        //~^ ERROR: left recursion: `a` -> `b` -> `a`
        b = "y"? a
    }
}

fn main() {}
