// Grammars inside function bodies.

mod lexical {
    prpr::grammar! {
        pub atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
    }
}

#[test]
fn grammar_in_a_function() {
    #[derive(Debug, PartialEq)]
    struct Pair(i64, i64);

    prpr::grammar! {
        pub pair: Pair = "(" a:crate::lexical::number "," b:number ")" { Pair(a, b) }

        atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
    }

    assert_eq!(pair("(1, 2)"), Ok(Pair(1, 2)));
    let (p, end) = prpr::runtime::parse_prefix("(3,4) rest", Rules::pair).unwrap();
    assert_eq!((p, end), (Pair(3, 4), 5));
    assert_eq!(number("5"), Ok(5));
}

mod visibility {
    pub mod inner {
        prpr::grammar! {
            pub(super) atom word: &'i str = s:([a-z]+) { #s }
        }
    }

    // A `pub(super)` rule is visible in the parent module, through `Rules` too.
    pub fn word(input: &str) -> Option<&str> {
        let (w, _) = prpr::runtime::parse_prefix(input, inner::Rules::word).ok()?;
        Some(w)
    }
}

#[test]
fn rules_have_the_visibility_of_their_rules() {
    assert_eq!(visibility::word("abc def"), Some("abc"));
}
