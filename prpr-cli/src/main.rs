//! The `prpr` command: generates parsers from grammar files.

use clap::{Parser, Subcommand};
use prpr_codegen::Diagnostic;
use prpr_codegen::file::Runtime;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Generates Packrat parsers from grammar files.
#[derive(Parser)]
#[command(name = "prpr", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generates a Rust module from a grammar file.
    Gen {
        /// The grammar file.
        grammar: PathBuf,
        /// Writes to OUTPUT instead of the standard output.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Refers to the runtime at PATH, e.g. a module written by `prpr runtime`.
        #[arg(long, value_name = "PATH", default_value = "::prpr_runtime")]
        runtime_path: String,
        /// Includes the runtime in the generated module.
        #[arg(long, conflicts_with = "runtime_path")]
        inline_runtime: bool,
    },
    /// Checks a grammar file.
    Check {
        /// The grammar file.
        grammar: PathBuf,
    },
    /// Writes the runtime as a module, for `prpr gen --runtime-path`.
    Runtime {
        /// Writes to OUTPUT instead of the standard output.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    match run(Cli::parse().command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}

// Errors are reported to the standard error before they are returned.
fn run(command: Command) -> Result<(), ()> {
    match command {
        Command::Gen {
            grammar,
            output,
            runtime_path,
            inline_runtime,
        } => {
            let runtime = if inline_runtime {
                Runtime::Inline
            } else {
                Runtime::Path(runtime_path)
            };
            let code = generate(&grammar, &runtime)?;
            write(output.as_deref(), &code)
        }
        // A grammar is checked by generating code from it.
        Command::Check { grammar } => {
            generate(&grammar, &Runtime::Inline)?;
            Ok(())
        }
        Command::Runtime { output } => {
            write(output.as_deref(), &prpr_codegen::file::runtime_module())
        }
    }
}

fn generate(path: &Path, runtime: &Runtime) -> Result<String, ()> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| eprintln!("error: cannot read {}: {e}", path.display()))?;
    prpr_codegen::file::generate(&text, runtime).map_err(|diags| report(&diags, &text, path))
}

fn report(diags: &[Diagnostic], text: &str, path: &Path) {
    for d in diags {
        eprintln!("{}\n", d.render(text, &path.display().to_string()));
    }
}

fn write(output: Option<&Path>, content: &str) -> Result<(), ()> {
    match output {
        Some(path) => std::fs::write(path, content)
            .map_err(|e| eprintln!("error: cannot write {}: {e}", path.display())),
        None => {
            print!("{content}");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli() {
        Cli::command().debug_assert();
    }
}
