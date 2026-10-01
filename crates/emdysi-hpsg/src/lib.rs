//! A DELPH-IN compatible HPSG processor: type hierarchy, typed feature
//! structures and unification, built from grammars written in TDL.

mod bitset;
pub mod chartmap;
pub mod dag;
pub mod desc;
pub mod grammar;
pub mod lexicon;
pub mod morph;
pub mod parser;
pub mod types;
pub mod typesys;
pub mod unify;

pub use dag::Dag;
pub use grammar::{Grammar, GrammarError, Instance};
pub use types::{Hierarchy, HierarchyError, TOP, TypeId};
pub use typesys::{FeatId, Features, LiteralKind, TypeSystem};
pub use unify::{Failure, Unifier};

use emdysi_tdl::{DefOp, Entry, Env, Term};

/// Collect `(type, parents)` declarations from loaded TDL entries, merging
/// addenda (`:+`) into the type they extend.
pub fn type_declarations(entries: &[Entry]) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    let mut pos: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for e in entries.iter().filter(|e| e.env == Env::Type) {
        let name = e.def.name.to_lowercase();
        let parents: Vec<String> = e
            .def
            .body
            .0
            .iter()
            .filter_map(|t| match t {
                Term::Type(p) => Some(p.to_lowercase()),
                _ => None,
            })
            .collect();
        match (e.def.op, pos.get(&name)) {
            (DefOp::Addendum, Some(&i)) => out[i].1.extend(parents),
            (_, Some(&i)) => out[i].1.extend(parents),
            (_, None) => {
                pos.insert(name.clone(), out.len());
                out.push((name, parents));
            }
        }
    }
    out
}
