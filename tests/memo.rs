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

// A memoized rule reports the same errors as a plain one, even where the result was first
// computed in a context that recorded its failures differently.
mod plain_errors {
    prpr::grammar! {
        pub lookahead = !item "x" / item "!"
        pub labeled = item "!" / thing
        #[label = "thing"]
        thing = item "?"
        item = "a" "b"
    }
}

mod memo_errors {
    prpr::grammar! {
        pub lookahead = !item "x" / item "!"
        pub labeled = item "!" / thing
        #[label = "thing"]
        thing = item "?"
        #[memo]
        item = "a" "b"
    }
}

#[test]
fn memo_keeps_errors() {
    let message = |r: Result<(), prpr::ParseError>| r.unwrap_err().to_string();
    for input in ["", "a", "a c", "ab", "ab?", "c"] {
        assert_eq!(
            message(memo_errors::lookahead(input)),
            message(plain_errors::lookahead(input)),
            "{input:?}"
        );
        assert_eq!(
            message(memo_errors::labeled(input)),
            message(plain_errors::labeled(input)),
            "{input:?}"
        );
    }
    assert_eq!(
        message(memo_errors::lookahead("a c")),
        "expected `b`, found `c`"
    );
    assert_eq!(
        message(memo_errors::labeled("c")),
        "expected one of `a` or thing, found `c`"
    );
}
