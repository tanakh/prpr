# Error messages

When the input does not match, the generated functions return a `prpr::ParseError`.
This chapter explains what an error contains, how prpr finds it, how grammars can improve it, and how to display it.

## Parse errors

A `ParseError` says what was expected at a position of the input, and what was found there instead:

```rust
prpr::grammar! {
    pub list: Vec<i64> = "[" xs:(number, ",")* "]" { xs }

    #[label = "number"]
    atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
}

fn main() {
    let e = list("[1, 2").unwrap_err();
    assert_eq!(e.to_string(), "expected one of `,` or `]`, found end of input");
    assert_eq!(e.offset(), 5);
    assert_eq!(e.span(), std::range::Range::from(5..5));
    assert_eq!(e.expected(), [prpr::Expected::Literal(","), prpr::Expected::Literal("]")]);
    assert_eq!(e.found(), prpr::Found::Eof);
}
```

* `offset()` is the byte offset of the error, and `span()` the span of what was found there (empty at the end of the input), as a `std::range::Range<usize>`.
* `expected()` lists what was expected, sorted and without duplicates.
  Each item is a `prpr::Expected`: a literal, a byte string, a character class, a label, any character, the end of the input, a delimiter, or a message from a semantic predicate.
* `found()` is the character (or byte) at the offset, or the end of the input.
* `Display` writes "expected ..., found ...", or "unexpected ..." when nothing in particular was expected, as when a predicate returned `None` or `false`.

## How errors are found

A PEG parser tries alternatives and backtracks, so many things fail while parsing even valid input.
The failure that matters is the one at the farthest position the parser reached, since everything before it matched.
prpr reports that position, with everything that was expected there.

A few rules keep the list relevant:

* Whitespace and comments, the rules `spaces`, `whitespace` and `comment`, are never reported as expected, except for the end of a comment that started and cannot end.
* When an atom (or an atomic group) is used as a token and matches, what was expected inside it is forgotten, as if a lexer had read the token.
  After the `2` in `[1, 2`, the error above does not list digits, even though more digits would have been accepted.
  When the token does not match, what was expected inside it is kept, so that, for example, an unterminated string is reported at its end.
* A labeled rule reports its label, as explained next.

Recording what was expected takes time, so a parse first runs without recording anything.
Only when it fails is the input parsed again, recording what was expected.
Parsing valid input is therefore not slowed down by error reporting, but actions may run twice for invalid input.

## Labels

`#[label = "..."]` gives a rule a name for error messages.
When the rule fails at the position where it starts, the error says its label instead of everything that was expected inside it:

```rust
prpr::grammar! {
    pub list = "[" (item, ",")* "]"

    #[label = "item"]
    item = "<" [a-z]+ ">"
}

fn main() {
    let message = |s| list(s).unwrap_err().to_string();
    assert_eq!(message("[<a>, ]"), "expected item, found `]`");
    assert_eq!(message("[<a>, <b]"), "expected one of `>` or `[a-z]`, found `]`");
}
```

In the second case, the error is inside an `item`, past its start, so it is reported as it is.
When labeled rules start at the same position, the outermost label wins.

Good candidates for labels are tokens (`number`, `string`, `identifier`) and the main constructs of the language (`expression`, `statement`, `type`).

## Messages from predicates

A semantic predicate that returns `Err(message)` reports the message as what was expected, at the start of its sequence:

```rust
prpr::grammar! {
    pub port: u16 = s:digits &{ #s.parse::<u16>().map_err(|_| "a port number") }

    atom digits = [0-9]+
}

fn main() {
    let e = port("99999").unwrap_err();
    assert_eq!(e.to_string(), "expected a port number, found `9`");
    assert_eq!(e.offset(), 0);
}
```

The sequence matched, but it is not valid as a whole, so what was expected inside it is forgotten, and so is what other alternatives expected at the same place: the message says more about the input than they do.
A predicate that returns `None` or `false` only filters, and leaves the other errors as they are.

## Displaying errors

`render` displays an error with the line of the input it is on, in the style of rustc:

```rust
# prpr::grammar! {
#     pub list: Vec<i64> = "[" xs:(number, ",")* "]" { xs }
#
#     #[label = "number"]
#     atom number: i64 = s:([0-9]+) { #s.parse().unwrap() }
# }
fn main() {
    let input = "[1,\n 2,]";
    let e = list(input).unwrap_err();
    assert_eq!(e.line_col(input), prpr::LineCol { line: 2, column: 4 });
    assert_eq!(
        e.render(input, "input.json").to_string(),
        "\
error: expected number, found `]`
 --> input.json:2:4
  |
2 |  2,]
  |    ^"
    );
}
```

`line_col` gives the line and column (counted in characters, from 1) of the error.

### With error reporting libraries

Spans are byte ranges, which diagnostic libraries take as `std::ops::Range<usize>`; convert them with `std::ops::Range::from(e.span())`.
Each example below reports the error of `list("[1, 2,]")` from the beginning of this chapter, and is followed by its output in a terminal.

With [ariadne](https://crates.io/crates/ariadne) 0.6:

```rust,ignore
use ariadne::{Color, Label, Report, ReportKind, Source};

fn report(path: &str, src: &str, e: &prpr::ParseError) {
    let span = std::ops::Range::from(e.span());
    Report::build(ReportKind::Error, (path, span.clone()))
        .with_message(e.to_string())
        .with_label(Label::new((path, span)).with_message("here").with_color(Color::Red))
        .finish()
        .eprint((path, Source::from(src)))
        .unwrap();
}
```

<pre class="terminal"><code class="nohighlight"><span class="ansi-31">Error:</span> expected number, found `]`
   <span class="ansi-38-5-246">╭─[</span> input.txt:1:7 <span class="ansi-38-5-246">]</span>
   <span class="ansi-38-5-246">│</span>
 <span class="ansi-38-5-246">1 │</span> <span class="ansi-38-5-249">[1, 2,</span><span class="ansi-31">]</span>
 <span class="ansi-38-5-240">  │</span>       <span class="ansi-31">┬</span>
 <span class="ansi-38-5-240">  │</span>       <span class="ansi-31">╰──</span> here
<span class="ansi-38-5-246">───╯</span></code></pre>

With [miette](https://crates.io/crates/miette) 7:

```rust,ignore
use miette::{LabeledSpan, NamedSource, miette};

fn report(path: &str, src: &str, e: &prpr::ParseError) -> miette::Report {
    let span = std::ops::Range::from(e.span());
    miette!(labels = vec![LabeledSpan::at(span, "here")], "{e}")
        .with_source_code(NamedSource::new(path, src.to_string()))
}
```

<pre class="terminal"><code class="nohighlight">  <span class="ansi-31">×</span> expected number, found `]`
   ╭─[<span class="ansi-36 ansi-bold ansi-underline">input.txt:1:7</span>]
 <span class="ansi-dim">1</span> │ [1, 2,]
   · <span class="ansi-35 ansi-bold">      ┬</span>
   ·       <span class="ansi-35 ansi-bold">╰── here</span>
   ╰────</code></pre>

With [codespan-reporting](https://crates.io/crates/codespan-reporting) 0.13:

```rust,ignore
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFile;
use codespan_reporting::term::{self, termcolor::{ColorChoice, StandardStream}};

fn report(path: &str, src: &str, e: &prpr::ParseError) {
    let file = SimpleFile::new(path, src);
    let diagnostic = Diagnostic::error()
        .with_message(e.to_string())
        .with_labels(vec![Label::primary((), std::ops::Range::from(e.span()))]);
    let writer = StandardStream::stderr(ColorChoice::Auto);
    term::emit_to_write_style(&mut writer.lock(), &term::Config::default(), &file, &diagnostic)
        .unwrap();
}
```

<pre class="terminal"><code class="nohighlight"><span class="ansi-38-5-9 ansi-bold">error</span><span class="ansi-bold">: expected number, found `]`</span>
  <span class="ansi-36">┌─</span> input.txt:1:7
  <span class="ansi-36">│</span>
<span class="ansi-36">1</span> <span class="ansi-36">│</span> [1, 2,<span class="ansi-31">]</span>
  <span class="ansi-36">│</span>       <span class="ansi-31">^</span></code></pre>

To word messages differently, build them from `expected()` and `found()`.
More kinds of `Expected` may be added in later versions, so a `match` on it needs a `_` arm.
