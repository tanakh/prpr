//! The `prpr` command: generates parsers from grammar files.

use prpr_codegen::Diagnostic;
use prpr_codegen::file::Runtime;
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
Usage:
  prpr gen <GRAMMAR> [-o <OUTPUT>] [--runtime-path <PATH> | --inline-runtime]
      Generates a Rust module from a grammar file.
  prpr check <GRAMMAR>
      Checks a grammar file.
  prpr runtime [-o <OUTPUT>]
      Writes the runtime as a module, for --runtime-path.

Options:
  -o, --output <OUTPUT>    Write to OUTPUT instead of the standard output.
  --runtime-path <PATH>    Refer to the runtime at PATH (default: ::prpr_runtime).
  --inline-runtime         Include the runtime in the generated module.";

struct Args {
    command: String,
    grammar: Option<PathBuf>,
    output: Option<PathBuf>,
    runtime_path: Option<String>,
    inline_runtime: bool,
}

// `Ok(None)` asks for help.
fn parse_args() -> Result<Option<Args>, String> {
    let mut args = std::env::args().skip(1);
    let command = args.next().ok_or("no command given")?;
    if command == "-h" || command == "--help" || command == "help" {
        return Ok(None);
    }
    let mut parsed = Args {
        command,
        grammar: None,
        output: None,
        runtime_path: None,
        inline_runtime: false,
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" | "--output" => parsed.output = Some(args.next().ok_or("-o needs a file")?.into()),
            "--runtime-path" => {
                parsed.runtime_path = Some(args.next().ok_or("--runtime-path needs a path")?)
            }
            "--inline-runtime" => parsed.inline_runtime = true,
            "-h" | "--help" => return Ok(None),
            s if s.starts_with('-') => return Err(format!("unknown option `{s}`")),
            _ if parsed.grammar.is_none() => parsed.grammar = Some(arg.into()),
            _ => return Err(format!("unexpected argument `{arg}`")),
        }
    }
    if parsed.inline_runtime && parsed.runtime_path.is_some() {
        return Err("--inline-runtime and --runtime-path cannot be used together".into());
    }
    Ok(Some(parsed))
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(Some(args)) => args,
        Ok(None) => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Err(e) => {
            eprintln!("error: {e}\n\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}

fn run(args: &Args) -> Result<(), ()> {
    match args.command.as_str() {
        "gen" | "check" => {
            let path = args
                .grammar
                .as_ref()
                .ok_or_else(|| eprintln!("error: no grammar file given"))?;
            let text = std::fs::read_to_string(path)
                .map_err(|e| eprintln!("error: cannot read {}: {e}", path.display()))?;
            let code = generate(args, &text).map_err(|diags| report(&diags, &text, path))?;
            if args.command == "gen" {
                write(args, &code)?;
            }
            Ok(())
        }
        "runtime" => write(args, &prpr_codegen::file::runtime_module()),
        command => {
            eprintln!("error: unknown command `{command}`\n\n{USAGE}");
            Err(())
        }
    }
}

fn generate(args: &Args, text: &str) -> Result<String, Vec<Diagnostic>> {
    let runtime = match (&args.runtime_path, args.inline_runtime) {
        (Some(path), _) => Runtime::Path(path.clone()),
        (None, true) => Runtime::Inline,
        (None, false) => Runtime::Path("::prpr_runtime".to_string()),
    };
    prpr_codegen::file::generate(text, &runtime)
}

fn report(diags: &[Diagnostic], text: &str, path: &std::path::Path) {
    for d in diags {
        eprintln!("{}\n", d.render(text, &path.display().to_string()));
    }
}

fn write(args: &Args, content: &str) -> Result<(), ()> {
    match &args.output {
        Some(path) => std::fs::write(path, content)
            .map_err(|e| eprintln!("error: cannot write {}: {e}", path.display())),
        None => {
            print!("{content}");
            Ok(())
        }
    }
}
