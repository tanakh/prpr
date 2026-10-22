//! The builder for build scripts.

use prpr_codegen::build::Builder;
use std::path::PathBuf;

// An empty output directory.
fn out_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn modules_are_written_at_the_paths_of_their_grammars() {
    let dir = out_dir("build-ok");
    Builder::new()
        .grammar("tests/grammars/list.peg")
        .out_dir(&dir)
        .try_compile()
        .unwrap();
    let code = std::fs::read_to_string(dir.join("tests/grammars/list.peg.rs")).unwrap();
    assert!(code.contains("pub fn list<'i>"));
    assert!(code.contains("::prpr_runtime::"));
}

#[test]
fn errors_are_rendered() {
    let e = Builder::new()
        .grammar("tests/grammars/undefined.peg")
        .grammar("tests/grammars/missing.peg")
        .out_dir(out_dir("build-errors"))
        .try_compile()
        .unwrap_err()
        .to_string();
    assert!(
        e.starts_with("error: cannot find rule `numbr`\n --> tests/grammars/undefined.peg:1:30\n")
    );
    assert!(e.contains("error: cannot read tests/grammars/missing.peg: "));
}

#[test]
fn paths_must_stay_in_the_crate() {
    let e = Builder::new()
        .grammar("../list.peg")
        .out_dir(out_dir("build-parent"))
        .try_compile()
        .unwrap_err()
        .to_string();
    assert!(e.contains("without `..`"), "{e}");
}
