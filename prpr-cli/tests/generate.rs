//! Generating files from grammars.

use prpr_codegen::file::{Runtime, generate};

// The location of the first error.
fn error_location(grammar: &str) -> String {
    match generate(grammar, &Runtime::Inline) {
        Ok(_) => panic!("no errors"),
        Err(diags) => diags[0]
            .render(grammar, "g.peg")
            .lines()
            .nth(1)
            .unwrap()
            .to_string(),
    }
}

#[test]
fn invalid_rust_code_is_located() {
    // In an action.
    assert_eq!(
        error_location("a: i32 = \"x\" {\n    1 +\n}\n"),
        " --> g.peg:3:1"
    );
    // In a Rust item, with a multi-byte character before it.
    assert_eq!(
        error_location("// é\nfn f() { let = 1; }\na = \"x\"\n"),
        " --> g.peg:2:14"
    );
}
