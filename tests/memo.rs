// Memoization with `#[memo]`.

use std::cell::Cell;

thread_local! {
    static CALLS: Cell<usize> = const { Cell::new(0) };
}

fn count() -> usize {
    CALLS.with(|c| {
        c.set(c.get() + 1);
        c.get()
    })
}

mod plain {
    use super::count;
    prpr::grammar! {
        pub start = item "a" / item "b" / item "c"
        item: usize = "x" { count() }
    }
}

mod memo {
    use super::count;
    prpr::grammar! {
        pub start = item "a" / item "b" / item "c"
        #[memo]
        item: usize = "x" { count() }
    }
}

fn calls(f: impl FnOnce()) -> usize {
    CALLS.with(|c| c.set(0));
    f();
    CALLS.with(Cell::get)
}

#[test]
fn memo_avoids_reparsing() {
    assert_eq!(calls(|| plain::start("x c").unwrap()), 3);
    assert_eq!(calls(|| memo::start("x c").unwrap()), 1);
}

// Exponential without memoization.
mod nested {
    prpr::grammar! {
        pub expr: i64 = term
        #[memo]
        term: i64 = "(" e:term ")" "!" { e + 1 } / "(" e:term ")" { e } / "x" { 0 }
    }
}

#[test]
fn memo_makes_backtracking_linear() {
    let depth = 200;
    let input = format!("{}x{}", "(".repeat(depth), ")".repeat(depth));
    assert_eq!(nested::expr(&input), Ok(0));
}
