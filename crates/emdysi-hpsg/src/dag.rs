//! Immutable typed feature structures stored as compact DAGs.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::types::TypeId;
use crate::typesys::{FeatId, Features, TypeSystem};

#[derive(Clone, Copy, Debug)]
pub struct Node {
    pub ty: TypeId,
    pub arc_start: u32,
    pub arc_len: u32,
}

/// A typed feature structure. Node 0 is the root. Arcs of each node are
/// sorted by feature id; shared node indices express reentrancy.
#[derive(Clone, Debug)]
pub struct Dag {
    pub nodes: Vec<Node>,
    pub arcs: Vec<(FeatId, u32)>,
}

impl Dag {
    pub fn atomic(ty: TypeId) -> Dag {
        Dag {
            nodes: vec![Node {
                ty,
                arc_start: 0,
                arc_len: 0,
            }],
            arcs: Vec::new(),
        }
    }

    pub fn root_type(&self) -> TypeId {
        self.nodes[0].ty
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn arcs(&self, n: u32) -> &[(FeatId, u32)] {
        let node = &self.nodes[n as usize];
        &self.arcs[node.arc_start as usize..(node.arc_start + node.arc_len) as usize]
    }

    pub fn ty(&self, n: u32) -> TypeId {
        self.nodes[n as usize].ty
    }

    pub fn arc(&self, n: u32, f: FeatId) -> Option<u32> {
        let arcs = self.arcs(n);
        arcs.binary_search_by_key(&f, |a| a.0)
            .ok()
            .map(|i| arcs[i].1)
    }

    /// Follow a feature path from node `n`.
    pub fn follow(&self, mut n: u32, path: &[FeatId]) -> Option<u32> {
        for &f in path {
            n = self.arc(n, f)?;
        }
        Some(n)
    }

    /// Render in TDL-like notation with `#n` tags for reentrancies.
    pub fn display(&self, ts: &TypeSystem, feats: &Features) -> String {
        let mut incoming = vec![0u32; self.nodes.len()];
        incoming[0] = 1;
        for &(_, t) in &self.arcs {
            incoming[t as usize] += 1;
        }
        let mut out = String::new();
        let mut tags = HashMap::new();
        self.display_node(0, ts, feats, &incoming, &mut tags, 0, &mut out);
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn display_node(
        &self,
        n: u32,
        ts: &TypeSystem,
        feats: &Features,
        incoming: &[u32],
        tags: &mut HashMap<u32, usize>,
        indent: usize,
        out: &mut String,
    ) {
        if incoming[n as usize] > 1 {
            if let Some(t) = tags.get(&n) {
                let _ = write!(out, "#{t}");
                return;
            }
            let t = tags.len() + 1;
            tags.insert(n, t);
            let _ = write!(out, "#{t} & ");
        }
        out.push_str(&ts.name(self.ty(n)));
        let arcs = self.arcs(n);
        if arcs.is_empty() {
            return;
        }
        out.push_str(" [ ");
        let inner = indent + 2;
        for (i, &(f, v)) in arcs.iter().enumerate() {
            if i > 0 {
                out.push_str(",\n");
                out.push_str(&" ".repeat(inner));
            }
            let name = feats.name(f);
            out.push_str(name);
            out.push(' ');
            self.display_node(v, ts, feats, incoming, tags, inner + name.len() + 1, out);
        }
        out.push_str(" ]");
    }
}
