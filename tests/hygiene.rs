// Generated code does not depend on what names refer to where the grammar is.

// The variants are never used: neither the grammar nor the generated code refers to them.
#![allow(dead_code, unused_imports)]

mod shadowed {
    pub enum Shadow {
        Ok,
        Err,
        Some,
        None,
    }
    use Shadow::*;

    prpr::grammar! {
        pub list: Vec<u32> = "[" xs:(item, ",")* "]" { xs }
        #[memo]
        #[label = "item"]
        item: u32 = n:number &{ (n < 100).then_some(n) } / "x" { 0 }
        atom number: u32 = s:([0-9]+) &{ #s.parse::<u32>().map_err(|_| "a number") }
    }
}

#[test]
fn shadowed_prelude() {
    assert_eq!(shadowed::list("[1, x, 22]").unwrap(), vec![1, 0, 22]);
    assert!(shadowed::list("[100]").is_err());
}
