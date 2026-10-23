# Build scripts

A [build script](https://doc.rust-lang.org/cargo/reference/build-scripts.html) can generate the module of a grammar file at build time, so that the module is never out of date and does not need to be kept in the repository.
The `prpr-codegen` crate, the code generator behind the `prpr` command, does the work.

Compared with [`include_grammar!`](cli.md#including-a-grammar-file), which also generates a parser from a file at build time, the generated module is a file that can be read, the compiler reports errors in the Rust code of the grammar at their places in the module, and the crate does not depend on a proc-macro.

## Setting up

In `Cargo.toml`, the build script depends on `prpr-codegen` with the feature `build`, and the crate depends on the runtime:

```toml
[dependencies]
prpr-runtime = "0.1"

[build-dependencies]
prpr-codegen = { version = "0.1", features = ["build"] }
```

`build.rs` compiles the grammar:

```rust,ignore
fn main() {
    prpr_codegen::build::compile("src/list.peg");
}
```

The crate includes the generated module where it wants it, with the same path:

```rust,ignore
mod list {
    prpr_runtime::include_parser!("src/list.peg");
}

fn main() {
    println!("{:?}", list::list("[1, 2, 3]")); // Ok([1, 2, 3])
}
```

`compile` writes the module to the output directory of the build, at the path of the grammar with `.rs` appended (`$OUT_DIR/src/list.peg.rs`), and tells Cargo to run the build script again when the grammar changes.
`include_parser!` includes the module from there.
The path of a grammar is relative to the crate, and cannot contain `..`.

## Several grammars and options

`Builder` compiles several grammars, with options:

```rust,ignore
use prpr_codegen::build::Builder;
use prpr_codegen::file::Runtime;

fn main() {
    Builder::new()
        .grammar("src/expr.peg")
        .grammar("src/config.peg")
        .runtime(Runtime::Inline)
        .compile();
}
```

| method | does |
|---|---|
| `grammar(path)` | Adds a grammar file. |
| `runtime(runtime)` | Where the modules find the runtime: the `prpr-runtime` crate by default (`Runtime::Path("::prpr_runtime".into())`), a module at another path, such as `Runtime::Path("::prpr::runtime".into())` for a crate that depends on `prpr`, or a copy of the runtime in each module (`Runtime::Inline`), as described in [Where the runtime comes from](cli.md#where-the-runtime-comes-from). |
| `out_dir(dir)` | Writes the modules to `dir` instead of `$OUT_DIR`. |
| `compile()` | Generates the modules, and stops the build if a grammar has errors. |
| `try_compile()` | Generates the modules, and returns the errors instead. |

Use the same version of `prpr-codegen` and the runtime; the generated module checks that the runtime is compatible.

`include_parser!` is a macro of `prpr-runtime`, which a crate that depends on `prpr` finds as `prpr::runtime::include_parser!`.
It reads the module from `$OUT_DIR`, so a module written elsewhere with `out_dir` is included with `include!`.
With `Runtime::Inline`, a crate that depends on neither crate includes its modules with `include!` too:

```rust,ignore
mod list {
    include!(concat!(env!("OUT_DIR"), "/src/list.peg.rs"));
}
```

## Errors

When a grammar has mistakes, `compile` writes them to the standard error, with the lines of the grammar they are about, and the build fails.
Cargo shows them in its output:

```text
error: failed to run custom build command for `app v0.1.0 (/path/to/app)`

Caused by:
  process didn't exit successfully: `/path/to/app/target/debug/build/app-.../build-script-build` (exit status: 1)
  --- stdout
  cargo::rerun-if-changed=src/list.peg

  --- stderr
  error: cannot find rule `numbr`
   --> src/list.peg:2:30
    |
  2 | pub list: Vec<i64> = "[" xs:(numbr, ",")* "]" { xs }
    |                              ^^^^^
    = note: a rule with a similar name exists: `number`
```

Errors in the Rust code of actions that only the compiler finds, such as type errors, are reported in the generated module, under the output directory.

## Generating modules yourself

For other needs, such as writing modules elsewhere or processing them further, `prpr_codegen::file::generate` takes the text of a grammar and where the generated code finds the runtime, and returns the module as text, or the mistakes in the grammar as `Diagnostic`s:

```rust,ignore
use prpr_codegen::file::{Runtime, generate};

fn main() {
    let path = "src/list.peg";
    println!("cargo::rerun-if-changed={path}");
    let grammar = std::fs::read_to_string(path).unwrap();
    let code = generate(&grammar, &Runtime::Path("::prpr_runtime".into())).unwrap_or_else(|diagnostics| {
        for d in &diagnostics {
            eprintln!("{}\n", d.render(&grammar, path));
        }
        std::process::exit(1);
    });
    // ... write `code` where it is needed.
}
```

`prpr_codegen::file::runtime_module()` returns the runtime as the text of a module, as `prpr runtime` writes it.
