//! Variable property mappings (VPM): how grammar-internal variable types
//! and properties map to the MRS interface (e.g. `PNG.PN 3s` to
//! `PERS 3 NUM sg`). Only the grammar-to-MRS direction is implemented.
//!
//! A VPM file has a type section (`event <> e`) followed by property
//! sections, each headed `LEFT PATHS : RIGHT NAMES` and holding rules
//! `values OP values [conditions]`. Rules of a section are tried in order
//! and the first match produces the output properties; `*` matches any
//! value, `!` no value (on the left) or means "omit" (on the right).

use crate::Grammar;
use crate::dag::Dag;
use crate::types::TypeId;
use crate::typesys::FeatId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dir {
    Both,
    Forward,
    Backward,
}

#[derive(Debug, Clone)]
struct Rule {
    left: Vec<String>,
    right: Vec<String>,
    dir: Dir,
    /// Variable-type letters the rule applies to (empty: all).
    types: Vec<String>,
}

#[derive(Debug, Clone)]
struct Section {
    paths: Vec<Vec<FeatId>>,
    names: Vec<String>,
    rules: Vec<Rule>,
}

#[derive(Debug, Clone, Default)]
pub struct Vpm {
    types: Vec<Rule>,
    sections: Vec<Section>,
}

fn parse_rule(line: &str) -> Option<Rule> {
    let toks: Vec<&str> = line.split_whitespace().collect();
    let op = toks
        .iter()
        .position(|t| matches!(*t, "<>" | ">>" | "<<" | "=" | "==" | "<=" | "=>"))?;
    let dir = match toks[op] {
        ">>" | "=>" => Dir::Forward,
        "<<" | "<=" => Dir::Backward,
        _ => Dir::Both,
    };
    let mut right = Vec::new();
    let mut types = Vec::new();
    for t in &toks[op + 1..] {
        if let Some(inner) = t.strip_prefix('[').and_then(|t| t.strip_suffix(']')) {
            types.extend(inner.split_whitespace().map(String::from));
        } else {
            right.push(t.to_string());
        }
    }
    Some(Rule {
        left: toks[..op].iter().map(|s| s.to_string()).collect(),
        right,
        dir,
        types,
    })
}

impl Vpm {
    pub fn parse(g: &Grammar, src: &str) -> Vpm {
        let mut vpm = Vpm::default();
        let mut current: Option<Section> = None;
        for line in src.lines() {
            let line = line.split(';').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            if let Some((l, r)) = line.split_once(" : ") {
                if let Some(s) = current.take() {
                    vpm.sections.push(s);
                }
                current = Some(Section {
                    paths: l
                        .split_whitespace()
                        .map(|p| g.path(p).unwrap_or_default())
                        .collect(),
                    names: r.split_whitespace().map(String::from).collect(),
                    rules: Vec::new(),
                });
                continue;
            }
            let Some(rule) = parse_rule(line) else {
                continue;
            };
            match &mut current {
                Some(s) => s.rules.push(rule),
                None => vpm.types.push(rule),
            }
        }
        if let Some(s) = current {
            vpm.sections.push(s);
        }
        vpm
    }

    /// `t` is equal to or more specific than the type named `name`.
    fn matches(g: &Grammar, t: TypeId, name: &str) -> bool {
        if name == "*" {
            return true;
        }
        match g.ts.hier.id(name) {
            Some(n) => g.ts.subsumed_by(t, n),
            None => {
                g.ts.literal_value(t)
                    .is_some_and(|(_, v)| v.eq_ignore_ascii_case(name))
            }
        }
    }

    /// The MRS variable type letter for a grammar type.
    pub fn var_type(&self, g: &Grammar, t: TypeId) -> String {
        for r in &self.types {
            if r.dir == Dir::Backward || r.left.len() != 1 || r.right.len() != 1 {
                continue;
            }
            if Self::matches(g, t, &r.left[0]) {
                return r.right[0].clone();
            }
        }
        "u".to_string()
    }

    /// The MRS properties of the variable at node `n` of `dag`, whose type
    /// letter is `letter`, in VPM order.
    pub fn properties(
        &self,
        g: &Grammar,
        dag: &Dag,
        n: u32,
        letter: &str,
    ) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for s in &self.sections {
            let values: Vec<Option<TypeId>> = s
                .paths
                .iter()
                .map(|p| {
                    if p.is_empty() {
                        None
                    } else {
                        dag.follow(n, p).map(|m| dag.ty(m))
                    }
                })
                .collect();
            if values.iter().all(Option::is_none) {
                continue;
            }
            for r in &s.rules {
                if r.dir == Dir::Backward || r.left.len() != values.len() {
                    continue;
                }
                if !r.types.is_empty() && !r.types.iter().any(|t| t == letter) {
                    continue;
                }
                let ok =
                    r.left
                        .iter()
                        .zip(&values)
                        .all(|(want, have)| match (want.as_str(), have) {
                            ("!", v) => v.is_none(),
                            (_, None) => false,
                            (w, Some(t)) => Self::matches(g, *t, w),
                        });
                if !ok {
                    continue;
                }
                for (i, v) in r.right.iter().enumerate() {
                    if v == "!" {
                        continue;
                    }
                    let Some(name) = s.names.get(i) else { continue };
                    let value = if v == "*" {
                        match values.get(i).copied().flatten() {
                            Some(t) => g.ts.name(t),
                            None => continue,
                        }
                    } else {
                        v.clone()
                    };
                    out.push((name.clone(), value));
                }
                break;
            }
        }
        out
    }
}
