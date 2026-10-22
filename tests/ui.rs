//! Compile-fail tests of `grammar!`: diagnostics must point at the right part of the grammar.
//!
//! Run `cargo test --test ui -- --bless` to update the expected outputs.

use ui_test::custom_flags::edition::Edition;
use ui_test::dependencies::DependencyBuilder;
use ui_test::{Config, run_tests};

fn main() -> ui_test::Result<()> {
    let mut config = Config::rustc("tests/ui");
    config.bless_command = Some("cargo test --test ui -- --bless".into());
    let defaults = config.comment_defaults.base();
    defaults.set_custom("edition", Edition("2024".into()));
    defaults.set_custom(
        "dependencies",
        DependencyBuilder {
            crate_manifest_path: "tests/ui-deps/Cargo.toml".into(),
            ..DependencyBuilder::default()
        },
    );
    run_tests(config)
}
