//! The type hierarchy and its greatest-lower-bound (GLB) closure.
//!
//! Every type is encoded by the set of *declared* types it subsumes
//! (Aït-Kaci et al. 1989). Two types unify to the type whose code is the
//! intersection of theirs; the closure step adds anonymous `glbtype`s until
//! every non-empty intersection is the code of some type, so the hierarchy is
//! a meet semi-lattice.

use std::collections::{HashMap, HashSet};
use std::hash::BuildHasherDefault;
use std::sync::RwLock;

use crate::unify::FxHasher;

use crate::bitset::BitSet;

pub type TypeId = u32;

pub const TOP: TypeId = 0;
pub const TOP_NAME: &str = "*top*";

#[derive(Debug)]
pub enum HierarchyError {
    UndefinedParent { ty: String, parent: String },
    Cycle(String),
}

impl std::fmt::Display for HierarchyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HierarchyError::UndefinedParent { ty, parent } => {
                write!(f, "type {ty} has undefined parent {parent}")
            }
            HierarchyError::Cycle(t) => write!(f, "cycle in type hierarchy at {t}"),
        }
    }
}

impl std::error::Error for HierarchyError {}

pub(crate) type GlbCache = HashMap<(TypeId, TypeId), Option<TypeId>, BuildHasherDefault<FxHasher>>;

pub struct Hierarchy {
    pub(crate) names: Vec<String>,
    pub(crate) index: HashMap<String, TypeId>,
    /// Number of declared types (including `*top*`); ids beyond are GLB types.
    pub(crate) n_declared: usize,
    pub(crate) codes: Vec<BitSet>,
    pub(crate) code_index: HashMap<BitSet, TypeId>,
    /// Ancestors (reflexive) of each type, as a bit set over all type ids.
    pub(crate) ancestors: Vec<BitSet>,
    pub(crate) parents: Vec<Vec<TypeId>>,
    pub(crate) children: Vec<Vec<TypeId>>,
    pub(crate) glb_cache: RwLock<GlbCache>,
}

impl Hierarchy {
    /// Build the hierarchy from `(name, parents)` pairs. Names are
    /// case-insensitive and stored in lower case. Types without parents are
    /// placed directly under `*top*`, which is implicit.
    pub fn build(decls: &[(String, Vec<String>)]) -> Result<Self, HierarchyError> {
        let mut names = vec![TOP_NAME.to_string()];
        let mut index = HashMap::new();
        index.insert(TOP_NAME.to_string(), TOP);
        for (name, _) in decls {
            let name = name.to_lowercase();
            if !index.contains_key(&name) {
                index.insert(name.clone(), names.len() as TypeId);
                names.push(name);
            }
        }
        let n = names.len();
        let mut parents: Vec<Vec<TypeId>> = vec![Vec::new(); n];
        for (name, ps) in decls {
            let id = index[&name.to_lowercase()] as usize;
            for p in ps {
                let p = p.to_lowercase();
                let Some(&pid) = index.get(&p) else {
                    return Err(HierarchyError::UndefinedParent {
                        ty: name.clone(),
                        parent: p,
                    });
                };
                if !parents[id].contains(&pid) {
                    parents[id].push(pid);
                }
            }
        }
        for ps in parents.iter_mut().skip(1) {
            if ps.is_empty() {
                ps.push(TOP);
            }
            // `foo := *top* & bar` is redundant; keep the more specific parent.
            if ps.len() > 1 {
                ps.retain(|&p| p != TOP);
            }
        }

        // Topological order of the declared types (parents before children).
        let order = topo_order(&names, &parents)?;

        // Codes of declared types: the set of declared descendants.
        let mut children: Vec<Vec<TypeId>> = vec![Vec::new(); n];
        for (c, ps) in parents.iter().enumerate() {
            for &p in ps {
                children[p as usize].push(c as TypeId);
            }
        }
        let mut codes: Vec<BitSet> = (0..n).map(|_| BitSet::new(n)).collect();
        for &t in order.iter().rev() {
            let t = t as usize;
            let mut code = BitSet::new(n);
            code.insert(t);
            for &c in &children[t] {
                code.union_with(&codes[c as usize]);
            }
            codes[t] = code;
        }

        let mut h = Hierarchy {
            names,
            index,
            n_declared: n,
            code_index: HashMap::new(),
            codes,
            ancestors: Vec::new(),
            parents: Vec::new(),
            children: Vec::new(),
            glb_cache: RwLock::new(HashMap::default()),
        };
        // Each declared type's code contains the type itself, so codes of
        // declared types are distinct.
        for (i, c) in h.codes.iter().enumerate() {
            h.code_index.insert(c.clone(), i as TypeId);
        }
        h.close();
        Ok(h)
    }

    /// Add GLB types until the codes are closed under non-empty intersection.
    fn close(&mut self) {
        let debug = std::env::var("EMDYSI_DEBUG_GLB").is_ok();
        // Pairs whose intersection was already looked at in earlier rounds.
        let mut checked: HashSet<(TypeId, TypeId), BuildHasherDefault<FxHasher>> =
            HashSet::default();
        loop {
            let t0 = std::time::Instant::now();
            self.compute_structure();
            if debug {
                eprintln!("structure {:?} ({} types)", t0.elapsed(), self.codes.len());
            }
            let t0 = std::time::Instant::now();
            let mut pairs: HashSet<(TypeId, TypeId), BuildHasherDefault<FxHasher>> =
                HashSet::default();
            for m in 0..self.codes.len() {
                if self.parents[m].len() < 2 {
                    continue;
                }
                let anc: Vec<usize> = self.ancestors[m].iter().filter(|&a| a != m).collect();
                for (i, &a) in anc.iter().enumerate() {
                    for &b in &anc[i + 1..] {
                        if self.ancestors[a].contains(b) || self.ancestors[b].contains(a) {
                            continue;
                        }
                        let pair = (a as TypeId, b as TypeId);
                        if !checked.contains(&pair) {
                            pairs.insert(pair);
                        }
                    }
                }
            }
            if debug {
                eprintln!("pairs {:?} ({})", t0.elapsed(), pairs.len());
            }
            // Sorted so that GLB type numbering is deterministic.
            let mut pairs: Vec<(TypeId, TypeId)> = pairs.into_iter().collect();
            pairs.sort_unstable();
            checked.extend(pairs.iter().copied());
            let mut added = false;
            for (a, b) in pairs {
                let code = self.codes[a as usize].intersection(&self.codes[b as usize]);
                if code.is_empty() || self.code_index.contains_key(&code) {
                    continue;
                }
                let id = self.codes.len() as TypeId;
                self.names
                    .push(format!("glbtype{}", id as usize - self.n_declared + 1));
                self.code_index.insert(code.clone(), id);
                self.codes.push(code);
                added = true;
            }
            if !added {
                return;
            }
        }
    }

    /// Recompute ancestors, immediate parents and children from the codes.
    fn compute_structure(&mut self) {
        let n = self.codes.len();
        // For each declared type o, the types whose code contains o.
        let mut containing: Vec<Vec<usize>> = vec![Vec::new(); self.n_declared];
        for (u, code) in self.codes.iter().enumerate() {
            for o in code.iter() {
                containing[o].push(u);
            }
        }
        let mut ancestors = Vec::with_capacity(n);
        for t in 0..n {
            let code = &self.codes[t];
            // Use the member contained in the fewest types as the probe.
            let probe = code.iter().min_by_key(|&o| containing[o].len()).unwrap();
            let mut anc = BitSet::new(n);
            for &u in &containing[probe] {
                if code.is_subset(&self.codes[u]) {
                    anc.insert(u);
                }
            }
            ancestors.push(anc);
        }
        // Immediate parents: take strict ancestors from the most specific
        // (smallest code) up; one not yet covered by a chosen parent's
        // ancestors is itself a parent.
        let sizes: Vec<usize> = self.codes.iter().map(BitSet::count).collect();
        let mut parents = vec![Vec::new(); n];
        for (t, ps) in parents.iter_mut().enumerate() {
            let mut cand: Vec<usize> = ancestors[t].iter().filter(|&u| u != t).collect();
            cand.sort_by_key(|&u| (sizes[u], u));
            let mut covered = BitSet::new(n);
            for u in cand {
                if !covered.contains(u) {
                    ps.push(u as TypeId);
                    covered.union_with(&ancestors[u]);
                }
            }
            ps.sort_unstable();
        }
        let mut children = vec![Vec::new(); n];
        for (c, ps) in parents.iter().enumerate() {
            for &p in ps {
                children[p as usize].push(c as TypeId);
            }
        }
        self.ancestors = ancestors;
        self.parents = parents;
        self.children = children;
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    pub fn n_declared(&self) -> usize {
        self.n_declared
    }

    pub fn is_glb_type(&self, t: TypeId) -> bool {
        t as usize >= self.n_declared
    }

    pub fn id(&self, name: &str) -> Option<TypeId> {
        self.index
            .get(name)
            .or_else(|| self.index.get(&name.to_lowercase()))
            .copied()
    }

    pub fn name(&self, t: TypeId) -> &str {
        &self.names[t as usize]
    }

    pub fn parents(&self, t: TypeId) -> &[TypeId] {
        &self.parents[t as usize]
    }

    pub fn children(&self, t: TypeId) -> &[TypeId] {
        &self.children[t as usize]
    }

    /// `a` is equal to or more specific than `b`.
    pub fn subsumed_by(&self, a: TypeId, b: TypeId) -> bool {
        self.ancestors[a as usize].contains(b as usize)
    }

    /// All ancestors of `t`, including `t`.
    pub fn ancestors(&self, t: TypeId) -> impl Iterator<Item = TypeId> + '_ {
        self.ancestors[t as usize].iter().map(|u| u as TypeId)
    }

    /// Type ids ordered so that every type comes after all its supertypes.
    pub fn topological(&self) -> Vec<TypeId> {
        let mut ids: Vec<TypeId> = (0..self.len() as TypeId).collect();
        ids.sort_by_key(|&t| std::cmp::Reverse(self.codes[t as usize].count()));
        ids
    }

    /// The greatest lower bound of two types, or `None` if they are
    /// incompatible.
    pub fn glb(&self, a: TypeId, b: TypeId) -> Option<TypeId> {
        if self.subsumed_by(a, b) {
            return Some(a);
        }
        if self.subsumed_by(b, a) {
            return Some(b);
        }
        let key = if a < b { (a, b) } else { (b, a) };
        if let Some(r) = self.glb_cache.read().unwrap().get(&key) {
            return *r;
        }
        let code = self.codes[a as usize].intersection(&self.codes[b as usize]);
        let r = if code.is_empty() {
            None
        } else {
            Some(*self.code_index.get(&code).expect("GLB closure is complete"))
        };
        self.glb_cache.write().unwrap().insert(key, r);
        r
    }
}

fn topo_order(names: &[String], parents: &[Vec<TypeId>]) -> Result<Vec<TypeId>, HierarchyError> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        None,
        Active,
        Done,
    }
    let n = names.len();
    let mut mark = vec![Mark::None; n];
    let mut order = Vec::with_capacity(n);
    for start in 0..n {
        if mark[start] != Mark::None {
            continue;
        }
        // Iterative DFS over parent links: emit a type after its parents.
        let mut stack: Vec<(usize, usize)> = vec![(start, 0)];
        mark[start] = Mark::Active;
        while let Some(&mut (t, ref mut i)) = stack.last_mut() {
            if *i < parents[t].len() {
                let p = parents[t][*i] as usize;
                *i += 1;
                match mark[p] {
                    Mark::None => {
                        mark[p] = Mark::Active;
                        stack.push((p, 0));
                    }
                    Mark::Active => return Err(HierarchyError::Cycle(names[p].clone())),
                    Mark::Done => {}
                }
            } else {
                mark[t] = Mark::Done;
                order.push(t as TypeId);
                stack.pop();
            }
        }
    }
    Ok(order)
}
