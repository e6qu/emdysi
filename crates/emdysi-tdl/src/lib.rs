//! Reader for the DELPH-IN Type Description Language (TDL).
//!
//! [`parse_str`] turns one source text into [`Statement`]s. [`load`] reads a
//! top-level file, follows `:include` directives and tracks the
//! `:begin`/`:end` environments, yielding every definition tagged with the
//! environment (type or instance, plus status) it appeared in.

mod ast;
mod error;
mod lexer;
mod parser;

use std::path::{Path, PathBuf};

pub use ast::*;
pub use error::{Error, Result};
pub use parser::parse_str;

/// The environment a definition was read in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Env {
    Type,
    /// An instance, with its `:status` (e.g. `lex-entry`, `rule`), if any.
    Instance(Option<String>),
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub env: Env,
    pub def: Definition,
}

#[derive(Debug, Clone)]
pub struct LetterSet {
    pub var: char,
    pub chars: Vec<char>,
    pub wild: bool,
}

/// Everything read from a top-level TDL file and its includes, in order.
#[derive(Debug, Default)]
pub struct Loaded {
    pub entries: Vec<Entry>,
    pub letter_sets: Vec<LetterSet>,
    /// Every file read, in load order.
    pub files: Vec<PathBuf>,
}

/// Load `path` and everything it includes. `env` is the environment in force
/// at the start of the file (use [`Env::Type`] for a grammar's top file).
pub fn load(path: &Path, env: Env) -> Result<Loaded> {
    let mut out = Loaded::default();
    let mut stack = vec![env];
    load_into(path, &mut stack, &mut out)?;
    Ok(out)
}

/// Load TDL from a string. `:include` directives are not allowed.
pub fn load_str(src: &str, name: &str, env: Env) -> Result<Loaded> {
    let mut out = Loaded::default();
    let mut stack = vec![env];
    apply_statements(parse_str(src, name)?, name, None, &mut stack, &mut out)?;
    Ok(out)
}

/// Resolve an `:include` name relative to the including file: names without
/// an extension get `.tdl` appended.
pub fn resolve_include(from: &Path, name: &str) -> PathBuf {
    let base = from.parent().unwrap_or(Path::new("."));
    let mut p = base.join(name);
    if p.extension().is_none() {
        p.set_extension("tdl");
    }
    p
}

fn load_into(path: &Path, stack: &mut Vec<Env>, out: &mut Loaded) -> Result<()> {
    let src = std::fs::read_to_string(path).map_err(|err| Error::Io {
        path: path.display().to_string(),
        err,
    })?;
    let name = path.display().to_string();
    out.files.push(path.to_path_buf());
    apply_statements(parse_str(&src, &name)?, &name, Some(path), stack, out)
}

fn apply_statements(
    statements: Vec<Statement>,
    name: &str,
    path: Option<&Path>,
    stack: &mut Vec<Env>,
    out: &mut Loaded,
) -> Result<()> {
    for st in statements {
        match st {
            Statement::Def(def) => {
                let env = stack.last().cloned().unwrap_or(Env::Type);
                out.entries.push(Entry { env, def });
            }
            Statement::LetterSet { var, chars, wild } => {
                out.letter_sets.push(LetterSet { var, chars, wild })
            }
            Statement::Begin { kind, status } => {
                let env = match kind.as_str() {
                    "type" => Env::Type,
                    "instance" => Env::Instance(status),
                    _ => {
                        return Err(Error::syntax(
                            name,
                            0,
                            format!("unknown environment :{kind}"),
                        ));
                    }
                };
                stack.push(env);
            }
            Statement::End { .. } => {
                if stack.len() <= 1 {
                    return Err(Error::syntax(name, 0, "unbalanced :end"));
                }
                stack.pop();
            }
            Statement::Include(inc) => match path {
                Some(path) => load_into(&resolve_include(path, &inc), stack, out)?,
                None => {
                    return Err(Error::syntax(name, 0, ":include is not allowed here"));
                }
            },
        }
    }
    Ok(())
}
