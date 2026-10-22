//! Generating parsers from grammar files in build scripts.
//!
//! ```no_run
//! // In `build.rs`:
//! prpr_codegen::build::compile("src/list.peg");
//! ```
//!
//! The crate then includes the module generated from the grammar, with the same path:
//!
//! ```ignore
//! mod list {
//!     prpr_runtime::include_parser!("src/list.peg");
//! }
//! ```

use crate::file::{Runtime, generate};
use std::fmt;
use std::path::{Component, Path, PathBuf};

/// Generates the module of the grammar file at `path`, relative to the crate, into `$OUT_DIR`.
/// If the grammar has errors, writes them to the standard error and stops the build.
pub fn compile(path: impl AsRef<Path>) {
    Builder::new().grammar(path).compile();
}

/// Generates the modules of grammar files, with options.
///
/// ```no_run
/// use prpr_codegen::build::Builder;
/// use prpr_codegen::file::Runtime;
///
/// Builder::new()
///     .grammar("src/expr.peg")
///     .grammar("src/config.peg")
///     .runtime(Runtime::Inline)
///     .compile();
/// ```
#[derive(Clone, Debug)]
pub struct Builder {
    grammars: Vec<PathBuf>,
    runtime: Runtime,
    out_dir: Option<PathBuf>,
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder {
    /// A builder without grammars, whose modules refer to the `prpr-runtime` crate.
    pub fn new() -> Self {
        Builder {
            grammars: Vec::new(),
            runtime: Runtime::Path("::prpr_runtime".into()),
            out_dir: None,
        }
    }

    /// Adds a grammar file, by its path relative to the crate, without `..`.
    /// Its module is written to that path with `.rs` appended, under the output directory.
    pub fn grammar(mut self, path: impl AsRef<Path>) -> Self {
        self.grammars.push(path.as_ref().to_path_buf());
        self
    }

    /// Where the modules find the runtime: the `prpr-runtime` crate by default.
    pub fn runtime(mut self, runtime: Runtime) -> Self {
        self.runtime = runtime;
        self
    }

    /// Writes the modules to `dir` instead of `$OUT_DIR`.
    pub fn out_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.out_dir = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Generates the modules, and tells Cargo to run the build script again when a grammar
    /// changes. If a grammar has errors, writes them to the standard error and stops the build.
    pub fn compile(&self) {
        if let Err(e) = self.try_compile() {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }

    /// Generates the modules, like [`Builder::compile`], but returns the errors.
    pub fn try_compile(&self) -> Result<(), Error> {
        let out_dir = match &self.out_dir {
            Some(dir) => dir.clone(),
            None => std::env::var_os("OUT_DIR")
                .map(PathBuf::from)
                .ok_or_else(|| {
                    Error(
                        "error: `OUT_DIR` is not set; grammars are compiled in build scripts"
                            .into(),
                    )
                })?,
        };
        let mut errors = Vec::new();
        for path in &self.grammars {
            println!("cargo::rerun-if-changed={}", path.display());
            if let Err(e) = self.compile_grammar(path, &out_dir) {
                errors.push(e);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error(errors.join("\n\n")))
        }
    }

    fn compile_grammar(&self, path: &Path, out_dir: &Path) -> Result<(), String> {
        let shown = path.display();
        // The module is written under `out_dir` at the same path, which must not leave it.
        if !path
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
        {
            return Err(format!(
                "error: `{shown}`: the path of a grammar must be relative to the crate, without `..`"
            ));
        }
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("error: cannot read {shown}: {e}"))?;
        let code = generate(&text, &self.runtime).map_err(|diags| {
            let rendered: Vec<String> = diags
                .iter()
                .map(|d| d.render(&text, &shown.to_string()))
                .collect();
            rendered.join("\n\n")
        })?;
        let mut name = path.as_os_str().to_owned();
        name.push(".rs");
        let out = out_dir.join(name);
        let write = |out: &Path| {
            if let Some(dir) = out.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(out, code)
        };
        write(&out).map_err(|e| format!("error: cannot write {}: {e}", out.display()))
    }
}

/// The errors of [`Builder::try_compile`], rendered with the lines of the grammars they are about.
#[derive(Debug)]
pub struct Error(String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}
