# prpr design note

## The goal

* Must be sufficiently fast.
* Must allow for intuitive syntax definition.
* Must support both text (`&str`) and binary (`&[u8]`) data.
* Should minimize cloning wherever possible.
* While it is a Packrat parser, it must allow for explicit control over which parts are cached—given that caching inherently introduces performance overhead.
  * Alternatively, it should be designed such that the decision to *not* cache is an informed, deliberate choice.
* Must be capable of handling delimiters and spacing effectively, even without a separate lexer (e.g., through the use of "atomic" directives).
* Must be able to generate error information suitable for diagnostic error reporting, with an API designed for easy integration with existing error-reporting frameworks.
* Must support code generation via `proc-macro`, as well as the generation of standalone Rust files.
* Ideally, it should also be capable of generating standalone source files in other programming languages.
* It should be suitable for generating parsers intended for use within `proc-macro`s.

## Development flow

First, write a naive parser—using a tool like winnow, for example. Then, gradually replace it with code generated from a self-describing parser.
I will figure out the specifics of how to bootstrap the process.

Perform `rustc`-style UI testing using tools such as the [`ui_test`](https://crates.io/crates/ui_test) crate.

## Syntax definition

* [Self-describing parser](prpr.peg)
