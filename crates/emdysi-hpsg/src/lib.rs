//! A DELPH-IN compatible HPSG processor: type hierarchy, typed feature
//! structures and unification, built from grammars written in TDL.

mod bitset;
pub mod cache;
pub mod chartmap;
pub mod dag;
pub mod desc;
pub mod grammar;
pub mod labels;
pub mod lexicon;
pub mod morph;
pub mod mrs;
pub mod parser;
pub mod types;
pub mod typesys;
pub mod unify;
pub mod vpm;

pub use dag::Dag;
pub use grammar::{Grammar, GrammarError, Instance};
pub use types::{Hierarchy, HierarchyError, TOP, TypeId};
pub use typesys::{FeatId, Features, LiteralKind, TypeSystem};
pub use unify::{Failure, Unifier};

use emdysi_tdl::{DefOp, Entry, Env, Term};

/// Collect `(type, parents)` declarations from loaded TDL entries, merging
/// addenda (`:+`) into the type they extend. A later definition (`:=`) of a
/// type replaces the earlier one, as in ACE: the ERG's grammar-error
/// variant redefines some types of the standard grammar this way (e.g.
/// `d_-_prt_le` in `educ/lextypes-educ.tdl`).
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
            (DefOp::Define, Some(&i)) => out[i].1 = parents,
            (_, Some(&i)) => out[i].1.extend(parents),
            (_, None) => {
                pos.insert(name.clone(), out.len());
                out.push((name, parents));
            }
        }
    }
    out
}
