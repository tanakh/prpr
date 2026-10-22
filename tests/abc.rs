// Languages that are not context-free, with lookahead.

prpr::grammar! {
    // { a^n b^n | n >= 1 }
    atom ab: i64 = "a" v:ab? "b" { v.unwrap_or(0) + 1 }

    // { a^n b^n c^n | n >= 1 }
    atom abc: i64 = &(a !"b") "a"+ v:b !. { v }
    atom a: i64 = "a" v:a? "b" { v.unwrap_or(0) + 1 }
    atom b: i64 = "b" v:b? "c" { v.unwrap_or(0) + 1 }
}

#[test]
fn anbn() {
    assert_eq!(ab("ab"), Ok(1));
    assert_eq!(ab("aabb"), Ok(2));
    assert_eq!(ab("aaabbb"), Ok(3));
    assert!(ab("aaabb").is_err());
    assert!(ab("bbaa").is_err());
}

#[test]
fn anbncn() {
    assert_eq!(abc("abc"), Ok(1));
    assert_eq!(abc("aabbcc"), Ok(2));
    assert_eq!(abc("aaabbbccc"), Ok(3));
    assert!(abc("aabbccc").is_err());
    assert!(abc("aaabbcc").is_err());
    assert!(abc("aabbbcc").is_err());
    assert!(abc("bbccaa").is_err());
}
