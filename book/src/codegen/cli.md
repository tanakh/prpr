# Grammar files and the prpr command

A grammar can also be written in a file of its own, from which the `prpr` command generates a Rust module.
A generated module is ordinary Rust code: it can be read, it builds without running a proc-macro, and the crate that uses it needs only the runtime, or nothing at all.
The price is a step to keep it up to date with the grammar, which a [build script](build-scripts.md) can take.

See [Installation](../getting-started/installation.md) for how to install the command.

## Grammar files

A grammar file contains what would be written inside `grammar!`: rules, Rust items and grammar attributes, with the same syntax.
Files are usually given the extension `.peg`:

```rust,ignore
// src/list.peg: a comma-separated list of numbers.
pub list: Vec<i64> = "[" xs:(number, ",")* "]" { xs }

#[label = "number"]
atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
```

## Generating a module

`prpr gen` generates a module from a grammar file:

```sh
prpr gen src/list.peg -o src/list.rs
```

The module is used like any other, here with `prpr-runtime` as a dependency of the crate:

```toml
[dependencies]
prpr-runtime = "0.1"
```

```rust,ignore
mod list;

fn main() {
    println!("{:?}", list::list("[1, 2, 3]")); // Ok([1, 2, 3])
}
```

The module has the same items as `grammar!` would generate (see [The generated API](api.md)), formatted for reading.
It starts with a comment saying that it was generated: edit the grammar and generate the module again, rather than editing the module.

Generated code, including the runtime that `--inline-runtime` and `prpr runtime` write, may be used under any terms, without the notices that the licenses of prpr require.
The comment at its start says so, and the [license](https://github.com/tanakh/prpr#license) of prpr states the exception.

## Where the runtime comes from

Generated code calls the runtime, which can be found in three ways:

| command | the module calls | the crate needs |
|---|---|---|
| `prpr gen list.peg` | the `prpr-runtime` crate | `prpr-runtime` as a dependency |
| `prpr gen list.peg --inline-runtime` | a copy of the runtime inside the module | nothing |
| `prpr gen list.peg --runtime-path crate::rt` | the module at `crate::rt` | nothing |

`--inline-runtime` makes a module that depends on nothing, which suits a crate with a single grammar.
The runtime is about 800 lines of code.

With several grammars, each would have its own copy, and the types of the copies, such as `ParseError`, would be different types.
Write the runtime once as a module of the crate instead, and refer to it from each generated module:

```sh
prpr runtime -o src/rt.rs
prpr gen src/list.peg --runtime-path crate::rt -o src/list.rs
prpr gen src/config.peg --runtime-path crate::rt -o src/config.rs
```

```rust,ignore
mod rt;
mod list;
mod config;
```

Grammars that [use each other's rules](../guide/composing.md) must use the same runtime.
A module that refers to a runtime outside itself checks at compile time that the version of the runtime is compatible with it, so update `src/rt.rs` or `prpr-runtime` together with the command.

## Checking a grammar

`prpr check` reports the mistakes in a grammar without writing anything:

```sh
prpr check src/list.peg
```

```text
error: cannot find rule `numbr`
 --> src/list.peg:2:30
  |
2 | pub list: Vec<i64> = "[" xs:(numbr, ",")* "]" { xs }
  |                              ^^^^^
  = note: a rule with a similar name exists: `number`
```

It exits with status 1 when there are errors, and so does `prpr gen`, which reports the same errors.
The [Diagnostics](../reference/diagnostics.md) chapter lists them.

Syntax errors in the Rust code of a grammar are reported at the grammar too.
The Rust code is otherwise checked by the compiler, which reports its errors, such as type errors in actions, in the generated module.

## Commands

| command | does |
|---|---|
| `prpr gen GRAMMAR` | Generates a module from `GRAMMAR`. |
| `prpr check GRAMMAR` | Checks `GRAMMAR`. |
| `prpr runtime` | Writes the runtime as a module, for `--runtime-path`. |
| `prpr help [COMMAND]` | Describes the commands and their options. |

| option | of | does |
|---|---|---|
| `-o`, `--output FILE` | `gen`, `runtime` | Writes to `FILE` instead of the standard output. |
| `--runtime-path PATH` | `gen` | Refers to the runtime at `PATH`. The default is `::prpr_runtime`, the crate. |
| `--inline-runtime` | `gen` | Includes the runtime in the module. |
