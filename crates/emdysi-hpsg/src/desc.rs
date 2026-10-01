//! Turning TDL descriptions into (not yet well-typed) DAGs.

use std::collections::HashMap;

use emdysi_tdl::{Conj, Term};

use crate::dag::{Dag, Node};
use crate::types::{TOP, TypeId};
use crate::typesys::{FeatId, Features, LiteralKind, TypeSystem};

/// Type and feature names used to encode lists and difference lists.
#[derive(Clone, Copy, Debug)]
pub struct ListConfig {
    pub list: TypeId,
    pub cons: TypeId,
    pub null: TypeId,
    pub diff_list: TypeId,
    pub first: FeatId,
    pub rest: FeatId,
    pub list_feat: FeatId,
    pub last: FeatId,
}

#[derive(Debug)]
pub struct DescError(pub String);

struct BNode {
    ty: TypeId,
    arcs: Vec<(FeatId, usize)>,
    fwd: Option<usize>,
}

/// Builds a DAG from TDL terms using union-find over description nodes.
pub struct DescBuilder<'a> {
    ts: &'a TypeSystem,
    feats: &'a mut Features,
    lists: ListConfig,
    nodes: Vec<BNode>,
    tags: HashMap<String, usize>,
}

impl<'a> DescBuilder<'a> {
    pub fn new(ts: &'a TypeSystem, feats: &'a mut Features, lists: ListConfig) -> Self {
        DescBuilder {
            ts,
            feats,
            lists,
            nodes: Vec::new(),
            tags: HashMap::new(),
        }
    }

    fn new_node(&mut self, ty: TypeId) -> usize {
        self.nodes.push(BNode {
            ty,
            arcs: Vec::new(),
            fwd: None,
        });
        self.nodes.len() - 1
    }

    fn find(&self, mut n: usize) -> usize {
        while let Some(f) = self.nodes[n].fwd {
            n = f;
        }
        n
    }

    fn constrain_type(&mut self, n: usize, ty: TypeId) -> Result<(), DescError> {
        let n = self.find(n);
        let cur = self.nodes[n].ty;
        match self.ts.glb(cur, ty) {
            Some(t) => {
                self.nodes[n].ty = t;
                Ok(())
            }
            None => Err(DescError(format!(
                "incompatible types {} and {}",
                self.ts.name(cur),
                self.ts.name(ty)
            ))),
        }
    }

    fn arc(&mut self, n: usize, f: FeatId) -> usize {
        let n = self.find(n);
        if let Some(&(_, v)) = self.nodes[n].arcs.iter().find(|a| a.0 == f) {
            return v;
        }
        let v = self.new_node(TOP);
        self.nodes[n].arcs.push((f, v));
        v
    }

    fn union(&mut self, a: usize, b: usize) -> Result<(), DescError> {
        let a = self.find(a);
        let b = self.find(b);
        if a == b {
            return Ok(());
        }
        let tb = self.nodes[b].ty;
        self.constrain_type(a, tb)?;
        self.nodes[b].fwd = Some(a);
        let arcs = std::mem::take(&mut self.nodes[b].arcs);
        for (f, v) in arcs {
            let a = self.find(a);
            match self.nodes[a].arcs.iter().find(|x| x.0 == f) {
                Some(&(_, w)) => self.union(w, v)?,
                None => self.nodes[a].arcs.push((f, v)),
            }
        }
        Ok(())
    }

    /// Add the constraints of `conj` to node `n`.
    pub fn add_conj(&mut self, n: usize, conj: &Conj) -> Result<(), DescError> {
        for term in &conj.0 {
            self.add_term(n, term)?;
        }
        Ok(())
    }

    fn add_term(&mut self, n: usize, term: &Term) -> Result<(), DescError> {
        match term {
            Term::Type(name) => {
                let t = self
                    .ts
                    .hier
                    .id(name)
                    .ok_or_else(|| DescError(format!("undefined type {name}")))?;
                self.constrain_type(n, t)
            }
            Term::Str(s) => {
                let t = self.ts.literal(LiteralKind::Str, s);
                self.constrain_type(n, t)
            }
            Term::Regex(s) => {
                let t = self.ts.literal(LiteralKind::Regex, s);
                self.constrain_type(n, t)
            }
            Term::Coref(tag) => {
                let tag = tag.to_lowercase();
                match self.tags.get(&tag) {
                    Some(&m) => self.union(n, m),
                    None => {
                        self.tags.insert(tag, n);
                        Ok(())
                    }
                }
            }
            Term::Avm(fvs) => {
                for fv in fvs {
                    let mut cur = n;
                    for f in &fv.path {
                        let f = self.feats.intern(f);
                        cur = self.arc(cur, f);
                    }
                    self.add_conj(cur, &fv.value)?;
                }
                Ok(())
            }
            Term::List { items, open, tail } => {
                let l = self.lists;
                let mut cur = n;
                for item in items {
                    self.constrain_type(cur, l.cons)?;
                    let first = self.arc(cur, l.first);
                    self.add_conj(first, item)?;
                    cur = self.arc(cur, l.rest);
                }
                if let Some(tail) = tail {
                    self.add_conj(cur, tail)
                } else if *open {
                    self.constrain_type(cur, l.list)
                } else {
                    self.constrain_type(cur, l.null)
                }
            }
            Term::DiffList(items) => {
                let l = self.lists;
                self.constrain_type(n, l.diff_list)?;
                let mut cur = self.arc(n, l.list_feat);
                for item in items {
                    self.constrain_type(cur, l.cons)?;
                    let first = self.arc(cur, l.first);
                    self.add_conj(first, item)?;
                    cur = self.arc(cur, l.rest);
                }
                let last = self.arc(n, l.last);
                self.union(cur, last)
            }
        }
    }

    /// Build a DAG whose root carries all of `conjs`; coreference tags are
    /// scoped to each conjunction (e.g. a definition and its addenda).
    pub fn build_all(mut self, conjs: &[&Conj]) -> Result<Dag, DescError> {
        let root = self.new_node(TOP);
        for conj in conjs {
            self.tags.clear();
            self.add_conj(root, conj)?;
        }
        self.finish(root)
    }

    /// Build a DAG whose root carries `conj`.
    pub fn build(mut self, conj: &Conj) -> Result<Dag, DescError> {
        let root = self.new_node(TOP);
        self.add_conj(root, conj)?;
        self.finish(root)
    }

    fn finish(self, root: usize) -> Result<Dag, DescError> {
        let mut map: HashMap<usize, u32> = HashMap::new();
        let mut order = Vec::new();
        let root = self.find(root);
        map.insert(root, 0);
        order.push(root);
        let mut i = 0;
        while i < order.len() {
            let n = order[i];
            for &(_, v) in &self.nodes[n].arcs {
                let v = self.find(v);
                if let std::collections::hash_map::Entry::Vacant(e) = map.entry(v) {
                    e.insert(order.len() as u32);
                    order.push(v);
                }
            }
            i += 1;
        }
        let mut dag = Dag {
            nodes: Vec::with_capacity(order.len()),
            arcs: Vec::new(),
        };
        for &n in &order {
            let start = dag.arcs.len();
            let mut arcs: Vec<(FeatId, u32)> = self.nodes[n]
                .arcs
                .iter()
                .map(|&(f, v)| (f, map[&self.find(v)]))
                .collect();
            arcs.sort_by_key(|a| a.0);
            dag.arcs.extend(arcs);
            dag.nodes.push(Node {
                ty: self.nodes[n].ty,
                arc_start: start as u32,
                arc_len: (dag.arcs.len() - start) as u32,
            });
        }
        Ok(dag)
    }
}
