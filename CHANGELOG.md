# Changelog

All notable changes to prpr are documented in this file.
The crates of the workspace (`prpr`, `prpr-runtime`, `prpr-macros`, `prpr-codegen` and `prpr-cli`) are released together, with the same version.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

The first release.
[The prpr Book](https://tanakh.github.io/prpr/) explains how to use it.
prpr requires Rust 1.96 or later.

### Added

#### Grammars

- Parsers generated from PEGs written in Rust code with `grammar!`, or in grammar files included with `include_grammar!`, with a function for each rule that parses a whole input, and the type `Rules` for Rust code that drives the parsing itself.
- Implicit whitespace: syntactic rules skip whitespace between tokens, while `atom` rules are tokens, which must be delimited, so that `123x` is not a number followed by `x`. Grammars can define `whitespace`, `comment`, `spaces` and `word_char`, and change the check after a token with `#[boundary(e)]` and `#[no_boundary]`.
- Values: rules with types, bindings (`x:e`), actions (`{ ... }`) that can use the text and span of bindings (`#x`, `@x`, `#_`, `@_`), and semantic predicates (`&{ ... }`) that filter, or reject input with a message.
- Text (`&str`) and binary (`#![input = bytes]`) input.
- Rules of other grammars, used by their paths.

#### Errors

- Parse errors that report what was expected at the farthest position reached, named with `#[label = "..."]` or by the messages of predicates, and `ParseError::render`, which shows them with the line of the input in the style of rustc.
- Failures are only recorded when the input is invalid: the input is parsed once without recording them, and again only if that fails.

#### Performance

- Memoization of the rules marked with `#[memo]`, and only those.
- `#![profile]` and `prpr::runtime::profile`, which count how often the rules are parsed, to find which ones to memoize.

#### Checks

- Mistakes in grammars, such as undefined rules (with suggestions of similar names), left recursion and repetitions of empty matches, reported as compile errors at the grammar.
- `#![lint]`, which reports alternatives that are never tried or never match, rules parsed more than once at the same position, and unused rules.

#### Generating code

- The `prpr` command (crate `prpr-cli`): `prpr gen` generates standalone Rust modules from grammar files, which find the runtime in the `prpr-runtime` crate, in a module written by `prpr runtime`, or in themselves; `prpr check` checks grammar files.
- `prpr_codegen::build` for build scripts, whose modules `prpr_runtime::include_parser!` includes, and `prpr_codegen::file` for other tools.
- `define_proc_macro!` and `SourceText` (feature `proc-macro2`), which define proc-macros that parse their input with a grammar, and report errors at the tokens of the input.
- The code that prpr generates, including the runtime, may be used under any terms.
