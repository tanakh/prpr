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

#[test]
fn unknown_bindings_are_located() {
    assert_eq!(
        error_location("a: String = x:\"a\" {\n    format!(\"{}{}\", #x, @y.start)\n}\n"),
        " --> g.peg:2:25"
    );
}

// All the errors of a grammar, as `prpr check` shows them.
fn errors(grammar: &str) -> String {
    match generate(grammar, &Runtime::Inline) {
        Ok(_) => panic!("no errors"),
        Err(diags) => {
            let rendered: Vec<String> = diags.iter().map(|d| d.render(grammar, "g.peg")).collect();
            rendered.join("\n\n")
        }
    }
}

#[test]
fn diagnostics_are_rendered_with_their_notes() {
    let grammar = "list = \"[\" (numbr, \",\")* \"]\"\natom number = [0-9]+\nlist = \"x\"\n";
    assert_eq!(
        errors(grammar),
        "\
error: rule `list` is defined more than once
 --> g.peg:3:1
  |
3 | list = \"x\"
  | ^^^^
note: the first definition is here
 --> g.peg:1:1
  |
1 | list = \"[\" (numbr, \",\")* \"]\"
  | ^^^^

error: cannot find rule `numbr`
 --> g.peg:1:13
  |
1 | list = \"[\" (numbr, \",\")* \"]\"
  |             ^^^^^
  = note: a rule with a similar name exists: `number`"
    );
}

#[test]
fn invalid_runtime_paths_are_reported() {
    for path in ["not a path!", "\"crate::rt", ""] {
        let e = generate("a = \"x\"", &Runtime::Path(path.into())).unwrap_err();
        assert!(
            e[0].message.starts_with("invalid runtime path"),
            "{path}: {}",
            e[0].message
        );
    }
    assert!(generate("a = \"x\"", &Runtime::Path("crate::rt".into())).is_ok());
}
