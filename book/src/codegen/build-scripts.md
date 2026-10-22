# Build scripts

A [build script](https://doc.rust-lang.org/cargo/reference/build-scripts.html) can generate the module of a grammar file at build time, so that the module is never out of date and does not need to be kept in the repository.
The `prpr-codegen` crate, the code generator behind the `prpr` command, does the work.

## Setting up

In `Cargo.toml`, the build script depends on `prpr-codegen`, with the feature `pretty` that generates modules as text, and the crate depends on the runtime:

```toml
[dependencies]
prpr-runtime = "0.1"

[build-dependencies]
prpr-codegen = { version = "0.1", features = ["pretty"] }
```

`build.rs` generates the module from `src/list.peg` into the output directory of the build:

```rust,ignore
use prpr_codegen::file::{Runtime, generate};
use std::path::PathBuf;

fn main() {
    let path = "src/list.peg";
    println!("cargo::rerun-if-changed={path}");
    let grammar = std::fs::read_to_string(path).unwrap();
    let runtime = Runtime::Path("::prpr_runtime".into());
    let code = generate(&grammar, &runtime).unwrap_or_else(|diagnostics| {
        for d in &diagnostics {
            eprintln!("{}\n", d.render(&grammar, path));
        }
        std::process::exit(1);
    });
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("list.rs");
    std::fs::write(out, code).unwrap();
}
```

The crate includes the generated module where it wants it:

```rust,ignore
mod list {
    include!(concat!(env!("OUT_DIR"), "/list.rs"));
}

fn main() {
    println!("{:?}", list::list("[1, 2, 3]")); // Ok([1, 2, 3])
}
```

`cargo::rerun-if-changed` tells Cargo to run the build script again only when the grammar changes.

## The code generator

`prpr_codegen::file::generate` takes the text of a grammar and where the generated code finds the runtime, and returns the module as text:

| `Runtime` | the module calls |
|---|---|
| `Runtime::Path(path)` | the runtime at `path`: `"::prpr_runtime"` for the crate, or a module written by `prpr_codegen::file::runtime_module()` |
| `Runtime::Inline` | a copy of the runtime inside the module, so that the crate needs no dependency |

These are the options of `prpr gen` described in [Grammar files and the prpr command](cli.md#where-the-runtime-comes-from).
Use the same version of `prpr-codegen` and `prpr-runtime`; the generated module checks that the runtime is compatible.

## Errors

When the grammar has mistakes, `generate` returns them as `Diagnostic`s, and `render` formats each with the line of the grammar it is about.
The build script above prints them and fails, and Cargo shows them in its output:

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
To read the module, write it into the source tree with `prpr gen` instead.
