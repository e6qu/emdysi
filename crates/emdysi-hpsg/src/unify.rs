//! Unification over immutable DAGs.
//!
//! A [`Unifier`] works on handles into one or more source DAGs and records
//! every change (forwarding, narrowed types, added arcs) in side tables
//! stamped with a generation counter, in the spirit of Tomabechi's
//! quasi-destructive unification. Sources are never modified, so they can be
//! shared between threads; a successful result is copied out into a new DAG.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::Arc;

use crate::dag::{Dag, Node};
use crate::types::TypeId;
use crate::typesys::{FeatId, TypeSystem};

const NONE: u32 = u32::MAX;

/// Looks up the expanded constraint of a type: `Ok(None)` when the
/// constraint adds nothing beyond the type itself, `Err(t)` when the
/// constraint of `t` is not available yet.
pub type ConstraintFn<'a> = dyn Fn(TypeId) -> Result<Option<Arc<Dag>>, TypeId> + 'a;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// The structures are incompatible.
    Clash,
    /// The constraint of this type is needed but not yet expanded.
    NeedConstraint(TypeId),
}

#[derive(Default)]
pub struct Unifier {
    srcs: Vec<(Arc<Dag>, u32)>,
    total: u32,
    generation: u32,
    stamp: Vec<u32>,
    fwd: Vec<u32>,
    ty: Vec<TypeId>,
    comp: Vec<u32>,
    copy: Vec<u32>,
    copy_stamp: Vec<u32>,
    copy_generation: u32,
    /// Added arcs: (feature, target handle, next arc in the node's list).
    comp_arcs: Vec<(FeatId, u32, u32)>,
    pub failure: Option<Failure>,
    /// GLBs of incomparable types already computed by this unifier, so the
    /// hot path avoids the type system's shared cache.
    glb_cache: HashMap<(TypeId, TypeId), Option<TypeId>, BuildHasherDefault<FxHasher>>,
}

/// A small, fast hasher for integer keys (after rustc's FxHasher).
#[derive(Default, Clone, Copy)]
pub struct FxHasher {
    hash: u64,
}

impl Hasher for FxHasher {
    fn finish(&self) -> u64 {
        self.hash
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u64(b as u64);
        }
    }

    fn write_u32(&mut self, i: u32) {
        self.write_u64(i as u64);
    }

    fn write_u64(&mut self, i: u64) {
        self.hash = (self.hash.rotate_left(5) ^ i).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }

    fn write_usize(&mut self, i: usize) {
        self.write_u64(i as u64);
    }
}

impl Unifier {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new unification, forgetting all sources and changes.
    pub fn begin(&mut self) {
        self.srcs.clear();
        self.comp_arcs.clear();
        self.total = 0;
        self.failure = None;
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.stamp.iter_mut().for_each(|s| *s = 0);
            self.generation = 1;
        }
    }

    /// Add a source DAG, returning the handle of its root.
    pub fn add(&mut self, dag: Arc<Dag>) -> u32 {
        let base = self.total;
        self.total += dag.nodes.len() as u32;
        let need = self.total as usize;
        if self.stamp.len() < need {
            let n = need.next_power_of_two();
            self.stamp.resize(n, 0);
            self.fwd.resize(n, NONE);
            self.ty.resize(n, NONE);
            self.comp.resize(n, NONE);
            self.copy.resize(n, NONE);
            self.copy_stamp.resize(n, 0);
        }
        self.srcs.push((dag, base));
        base
    }

    fn touch(&mut self, h: u32) {
        let i = h as usize;
        if self.stamp[i] != self.generation {
            self.stamp[i] = self.generation;
            self.fwd[i] = NONE;
            self.ty[i] = NONE;
            self.comp[i] = NONE;
        }
    }

    #[inline]
    fn fresh(&self, h: u32) -> bool {
        self.stamp[h as usize] == self.generation
    }

    #[inline]
    fn src(&self, h: u32) -> (usize, u32) {
        // Few sources: a linear scan from the end beats a binary search.
        let mut s = self.srcs.len() - 1;
        while self.srcs[s].1 > h {
            s -= 1;
        }
        (s, h - self.srcs[s].1)
    }

    fn src_node(&self, h: u32) -> (usize, Node) {
        let (s, local) = self.src(h);
        (s, self.srcs[s].0.nodes[local as usize])
    }

    #[inline]
    pub fn find(&self, mut h: u32) -> u32 {
        while self.fresh(h) && self.fwd[h as usize] != NONE {
            h = self.fwd[h as usize];
        }
        h
    }

    /// Current type of a node.
    pub fn node_type(&self, h: u32) -> TypeId {
        self.rep_type(self.find(h))
    }

    /// Current type of a representative node.
    #[inline]
    fn rep_type(&self, h: u32) -> TypeId {
        if self.fresh(h) && self.ty[h as usize] != NONE {
            return self.ty[h as usize];
        }
        self.src_node(h).1.ty
    }

    /// Force the type of a node, bypassing GLB computation.
    pub fn set_type(&mut self, h: u32, t: TypeId) {
        let h = self.find(h);
        self.touch(h);
        self.ty[h as usize] = t;
    }

    /// Value of feature `f` at node `h`.
    pub fn arc(&self, h: u32, f: FeatId) -> Option<u32> {
        self.rep_arc(self.find(h), f)
    }

    /// Value of feature `f` at representative node `h`.
    #[inline]
    fn rep_arc(&self, h: u32, f: FeatId) -> Option<u32> {
        let (s, node) = self.src_node(h);
        let dag = &self.srcs[s].0;
        let base = self.srcs[s].1;
        let arcs = &dag.arcs[node.arc_start as usize..(node.arc_start + node.arc_len) as usize];
        if let Ok(i) = arcs.binary_search_by_key(&f, |a| a.0) {
            return Some(base + arcs[i].1);
        }
        if self.fresh(h) {
            let mut c = self.comp[h as usize];
            while c != NONE {
                let (cf, target, next) = self.comp_arcs[c as usize];
                if cf == f {
                    return Some(target);
                }
                c = next;
            }
        }
        None
    }

    pub fn follow(&self, mut h: u32, path: &[FeatId]) -> Option<u32> {
        for &f in path {
            h = self.arc(h, f)?;
        }
        Some(self.find(h))
    }

    /// All arcs of representative node `h`, sorted by feature.
    pub fn arcs(&self, h: u32) -> Vec<(FeatId, u32)> {
        let mut out = Vec::new();
        self.arcs_into(h, &mut out);
        out
    }

    fn arcs_into(&self, h: u32, out: &mut Vec<(FeatId, u32)>) {
        out.clear();
        let (s, node) = self.src_node(h);
        let dag = &self.srcs[s].0;
        let base = self.srcs[s].1;
        out.extend(
            dag.arcs[node.arc_start as usize..(node.arc_start + node.arc_len) as usize]
                .iter()
                .map(|&(f, v)| (f, base + v)),
        );
        if self.fresh(h) && self.comp[h as usize] != NONE {
            let mut c = self.comp[h as usize];
            while c != NONE {
                let (f, target, next) = self.comp_arcs[c as usize];
                out.push((f, target));
                c = next;
            }
            out.sort_unstable_by_key(|a| a.0);
        }
    }

    fn push_comp(&mut self, h: u32, f: FeatId, target: u32) {
        self.touch(h);
        let idx = self.comp_arcs.len() as u32;
        self.comp_arcs.push((f, target, self.comp[h as usize]));
        self.comp[h as usize] = idx;
    }

    /// Unify the nodes `a` and `b`. On failure, [`Unifier::failure`] says why
    /// and the unifier must be restarted with [`Unifier::begin`].
    pub fn unify(&mut self, a: u32, b: u32, ts: &TypeSystem, cons: &ConstraintFn) -> bool {
        let a = self.find(a);
        let b = self.find(b);
        if a == b {
            return true;
        }
        let ta = self.rep_type(a);
        let tb = self.rep_type(b);
        let Some(t) = self.glb(ts, ta, tb) else {
            self.failure = Some(Failure::Clash);
            return false;
        };
        self.touch(a);
        self.touch(b);
        self.fwd[b as usize] = a;
        self.ty[a as usize] = t;
        if t != ta && t != tb {
            match cons(t) {
                Err(need) => {
                    self.failure = Some(Failure::NeedConstraint(need));
                    return false;
                }
                Ok(Some(c)) => {
                    let r = self.add(c);
                    if !self.unify(a, r, ts, cons) {
                        return false;
                    }
                }
                Ok(None) => {}
            }
        }
        // Move b's arcs over to a: first the source arcs, then added ones.
        let (s, node) = self.src_node(b);
        let base = self.srcs[s].1;
        for i in node.arc_start..node.arc_start + node.arc_len {
            let (f, v) = self.srcs[s].0.arcs[i as usize];
            if !self.merge_arc(a, f, base + v, ts, cons) {
                return false;
            }
        }
        let mut c = self.comp[b as usize];
        while c != NONE {
            let (f, v, next) = self.comp_arcs[c as usize];
            if !self.merge_arc(a, f, v, ts, cons) {
                return false;
            }
            c = next;
        }
        true
    }

    fn glb(&mut self, ts: &TypeSystem, a: TypeId, b: TypeId) -> Option<TypeId> {
        if a == b {
            return Some(a);
        }
        if TypeSystem::is_literal(a) || TypeSystem::is_literal(b) {
            return ts.glb(a, b);
        }
        if ts.hier.subsumed_by(a, b) {
            return Some(a);
        }
        if ts.hier.subsumed_by(b, a) {
            return Some(b);
        }
        let key = if a < b { (a, b) } else { (b, a) };
        if let Some(&r) = self.glb_cache.get(&key) {
            return r;
        }
        let r = ts.glb(a, b);
        self.glb_cache.insert(key, r);
        r
    }

    fn merge_arc(
        &mut self,
        a: u32,
        f: FeatId,
        vb: u32,
        ts: &TypeSystem,
        cons: &ConstraintFn,
    ) -> bool {
        let a = self.find(a);
        match self.rep_arc(a, f) {
            Some(va) => self.unify(va, vb, ts, cons),
            None => {
                self.push_comp(a, f, vb);
                true
            }
        }
    }

    /// Copy the structure reachable from `root` into a new DAG, leaving out
    /// the root features in `drop`. Returns `None` if the result is cyclic.
    pub fn copy(&mut self, root: u32, drop: &[FeatId]) -> Option<Dag> {
        let root = self.find(root);
        let mut order = vec![root];
        // Each copy gets its own stamp so several copies can be taken from
        // one unification state.
        self.copy_generation = self.copy_generation.wrapping_add(1);
        if self.copy_generation == 0 {
            self.copy_stamp.iter_mut().for_each(|s| *s = 0);
            self.copy_generation = 1;
        }
        let cg = self.copy_generation;
        self.copy_stamp[root as usize] = cg;
        self.copy[root as usize] = 0;
        let mut dag = Dag {
            nodes: Vec::new(),
            arcs: Vec::new(),
        };
        let mut buf = Vec::new();
        let mut i = 0;
        while i < order.len() {
            let h = order[i];
            self.arcs_into(h, &mut buf);
            let start = dag.arcs.len() as u32;
            for &(f, v) in &buf {
                if i == 0 && drop.contains(&f) {
                    continue;
                }
                let v = self.find(v);
                if self.copy_stamp[v as usize] != cg {
                    self.copy_stamp[v as usize] = cg;
                    self.copy[v as usize] = order.len() as u32;
                    order.push(v);
                }
                dag.arcs.push((f, self.copy[v as usize]));
            }
            dag.nodes.push(Node {
                ty: self.node_type(h),
                arc_start: start,
                arc_len: dag.arcs.len() as u32 - start,
            });
            i += 1;
        }
        if is_cyclic(&dag) { None } else { Some(dag) }
    }
}

/// Whether a DAG contains a cycle.
pub fn is_cyclic(dag: &Dag) -> bool {
    // 0 = unvisited, 1 = on the current path, 2 = done.
    let mut state = vec![0u8; dag.nodes.len()];
    let mut stack: Vec<(u32, u32)> = vec![(0, 0)];
    state[0] = 1;
    while let Some(&mut (n, ref mut i)) = stack.last_mut() {
        let arcs = dag.arcs(n);
        if (*i as usize) < arcs.len() {
            let v = arcs[*i as usize].1;
            *i += 1;
            match state[v as usize] {
                0 => {
                    state[v as usize] = 1;
                    stack.push((v, 0));
                }
                1 => return true,
                _ => {}
            }
        } else {
            state[n as usize] = 2;
            stack.pop();
        }
    }
    false
}
