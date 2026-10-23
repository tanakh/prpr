// Slices and spans of bindings in actions.

use std::range::Range;

prpr::grammar! {
    decimal: (String, Range<usize>) = s:<[1-9] [0-9]*> { (#s.to_string(), @s) }

    decimals: (String, Range<usize>) = decimal decimal decimal { (#_.to_string(), @_) }

    pair: (&'i str, Range<usize>, &'i str, Range<usize>) = "(" a:word "," b:word ")" { (#a, @a, #b, @b) }

    atom word = [a-z]+

    // A syntactic rule in an atom skips whitespace itself.
    atom interpolation: (&'i str, Range<usize>) = "${" e:sum "}" { (#e, @e) }
    sum = word "+" word
}

#[test]
fn binding() {
    assert_eq!(decimal("123"), Ok(("123".into(), Range::from(0..3))));
    assert_eq!(decimal(" 123 "), Ok(("123".into(), Range::from(1..4))));
    assert!(decimal("123abc").is_err());
    assert!(decimal("0").is_err());
}

#[test]
fn whole() {
    assert_eq!(
        decimals("123 456 789"),
        Ok(("123 456 789".into(), Range::from(0..11)))
    );
    assert_eq!(
        decimals("   123 456 789   "),
        Ok(("123 456 789".into(), Range::from(3..14)))
    );
}

#[test]
fn no_leading_whitespace() {
    assert_eq!(
        pair("( foo ,  bar )"),
        Ok(("foo", Range::from(2..5), "bar", Range::from(9..12)))
    );
    assert_eq!(
        interpolation("${  a + b}"),
        Ok(("a + b", Range::from(4..9)))
    );
}
