// Slices and spans of bindings in actions.

use prpr::Span;

prpr::grammar! {
    decimal: (String, Span) = s:<[1-9] [0-9]*> { (#s.to_string(), @s) }

    decimals: (String, Span) = decimal decimal decimal { (#_.to_string(), @_) }

    pair: (&'i str, Span, &'i str, Span) = "(" a:word "," b:word ")" { (#a, @a, #b, @b) }

    atom word = [a-z]+
}

#[test]
fn binding() {
    assert_eq!(decimal("123"), Ok(("123".into(), Span::new(0, 3))));
    assert_eq!(decimal(" 123 "), Ok(("123".into(), Span::new(1, 4))));
    assert!(decimal("123abc").is_err());
    assert!(decimal("0").is_err());
}

#[test]
fn whole() {
    assert_eq!(
        decimals("123 456 789"),
        Ok(("123 456 789".into(), Span::new(0, 11)))
    );
    assert_eq!(
        decimals("   123 456 789   "),
        Ok(("123 456 789".into(), Span::new(3, 14)))
    );
}

#[test]
fn no_leading_whitespace() {
    assert_eq!(
        pair("( foo ,  bar )"),
        Ok(("foo", Span::new(2, 5), "bar", Span::new(9, 12)))
    );
}
