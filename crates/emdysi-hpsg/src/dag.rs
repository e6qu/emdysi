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

impl Dag {
    /// A copy without the arcs labelled with any of `drop`, at any depth,
    /// keeping only the nodes still reachable.
    pub fn restrict(&self, drop: &[FeatId]) -> Dag {
        let mut map = vec![u32::MAX; self.nodes.len()];
        let mut order = vec![0u32];
        map[0] = 0;
        let mut out = Dag {
            nodes: Vec::with_capacity(self.nodes.len()),
            arcs: Vec::with_capacity(self.arcs.len()),
        };
        let mut i = 0;
        while i < order.len() {
            let n = order[i];
            let start = out.arcs.len() as u32;
            for &(f, v) in self.arcs(n) {
                if drop.contains(&f) {
                    continue;
                }
                if map[v as usize] == u32::MAX {
                    map[v as usize] = order.len() as u32;
                    order.push(v);
                }
                out.arcs.push((f, map[v as usize]));
            }
            out.nodes.push(Node {
                ty: self.ty(n),
                arc_start: start,
                arc_len: out.arcs.len() as u32 - start,
            });
            i += 1;
        }
        out
    }
}

/// Two-way subsumption: `(a subsumes b, b subsumes a)`, where "subsumes"
/// means "is equally or more general".
pub fn subsumes_both(ts: &TypeSystem, a: &Dag, b: &Dag) -> (bool, bool) {
    Subsumer::default().check(ts, a, b)
}

/// Reusable scratch space for subsumption checks.
#[derive(Default)]
pub struct Subsumer {
    amap: Vec<u32>,
    astamp: Vec<u32>,
    bmap: Vec<u32>,
    bstamp: Vec<u32>,
    generation: u32,
    stack: Vec<(u32, u32)>,
}

impl Subsumer {
    fn prepare(&mut self, na: usize, nb: usize) {
        if self.amap.len() < na {
            self.amap.resize(na, 0);
            self.astamp.resize(na, 0);
        }
        if self.bmap.len() < nb {
            self.bmap.resize(nb, 0);
            self.bstamp.resize(nb, 0);
        }
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.astamp.iter_mut().for_each(|x| *x = 0);
            self.bstamp.iter_mut().for_each(|x| *x = 0);
            self.generation = 1;
        }
        self.stack.clear();
    }

    /// `(a subsumes b, b subsumes a)`.
    pub fn check(&mut self, ts: &TypeSystem, a: &Dag, b: &Dag) -> (bool, bool) {
        self.check_restricted(ts, a, b, &[], false)
    }

    /// [`Subsumer::check`] as if both structures had been restricted
    /// ([`Dag::restrict`]): arcs whose feature is marked in `skip` are
    /// ignored at any depth, and with `any_root` the root types are not
    /// compared.
    pub fn check_restricted(
        &mut self,
        ts: &TypeSystem,
        a: &Dag,
        b: &Dag,
        skip: &[bool],
        any_root: bool,
    ) -> (bool, bool) {
        let skipped = |f: FeatId| skip.get(f as usize).copied().unwrap_or(false);
        self.prepare(a.nodes.len(), b.nodes.len());
        let g = self.generation;
        let mut fwd = true;
        let mut bwd = true;
        self.stack.push((0, 0));
        while let Some((x, y)) = self.stack.pop() {
            let (xi, yi) = (x as usize, y as usize);
            let a_seen = self.astamp[xi] == g;
            let b_seen = self.bstamp[yi] == g;
            // A pair already visited in both directions needs no second look.
            if a_seen && b_seen && self.amap[xi] == y && self.bmap[yi] == x {
                continue;
            }
            // A reentrancy present on one side only makes that side more specific.
            if !a_seen {
                self.astamp[xi] = g;
                self.amap[xi] = y;
            } else if self.amap[xi] != y {
                fwd = false;
            }
            if !b_seen {
                self.bstamp[yi] = g;
                self.bmap[yi] = x;
            } else if self.bmap[yi] != x {
                bwd = false;
            }
            let (tx, ty) = (a.ty(x), b.ty(y));
            if tx != ty && !(any_root && x == 0 && y == 0) {
                if !ts.subsumed_by(ty, tx) {
                    fwd = false;
                }
                if !ts.subsumed_by(tx, ty) {
                    bwd = false;
                }
            }
            if !fwd && !bwd {
                return (false, false);
            }
            let (ax, by) = (a.arcs(x), b.arcs(y));
            let (mut i, mut j) = (0, 0);
            loop {
                while i < ax.len() && skipped(ax[i].0) {
                    i += 1;
                }
                while j < by.len() && skipped(by[j].0) {
                    j += 1;
                }
                if i >= ax.len() && j >= by.len() {
                    break;
                }
                match (ax.get(i), by.get(j)) {
                    (Some(&(f, v)), Some(&(h, w))) if f == h => {
                        self.stack.push((v, w));
                        i += 1;
                        j += 1;
                    }
                    (Some(&(f, _)), Some(&(h, _))) if f < h => {
                        fwd = false;
                        i += 1;
                    }
                    (Some(_), None) => {
                        fwd = false;
                        i += 1;
                    }
                    _ => {
                        bwd = false;
                        j += 1;
                    }
                }
            }
            if !fwd && !bwd {
                return (false, false);
            }
        }
        (fwd, bwd)
    }
}
