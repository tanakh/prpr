// Counting the calls of rules.

prpr::grammar! {
    #![profile]
    // `term` is parsed twice at each level when there is no `!`; `memoized` is not.
    pub term = "(" term ")" "!" / "(" term ")" / "x"

    #[memo]
    pub memoized = "(" memoized ")" "!" / "(" memoized ")" / "x"
}

#[test]
fn profile_counts_calls() {
    let input = "((x))";
    let (result, profile) = prpr::runtime::profile(input, Rules::term);
    assert_eq!(result, Ok(((), 5)));
    let term = &profile.rules()[0];
    assert_eq!(term.rule, "term");
    // 1 call at depth 0, 2 at depth 1 and 4 at depth 2; all but 3 of them are repeated.
    assert_eq!((term.calls, term.repeated, term.memo_hits), (7, 4, 0));

    let (_, profile) = prpr::runtime::profile(input, Rules::memoized);
    let memoized = &profile.rules()[0];
    assert_eq!(
        (memoized.calls, memoized.repeated, memoized.memo_hits),
        (5, 2, 2)
    );
    assert_eq!(memoized.reparsed(), 0);
    let table = profile.to_string();
    assert!(table.starts_with("rule"), "{table}");
    assert!(table.contains("memoized"), "{table}");
}

#[test]
fn parsing_counts_too() {
    // The rules count their calls whenever they run.
    assert!(term("(x)").is_ok());
}
