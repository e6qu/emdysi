//! Compiling TDL into a grammar: the type system, expanded (well-typed)
//! type constraints, and instance descriptions.

use std::collections::HashMap;
use std::sync::Arc;

use emdysi_tdl::{Conj, Env};

use crate::dag::Dag;
use crate::desc::{DescBuilder, ListConfig};
use crate::types::{Hierarchy, TOP, TypeId};
use crate::typesys::{FeatId, Features, TypeSystem};
use crate::unify::{Failure, Unifier};

#[derive(Debug, Clone)]
pub struct GrammarError {
    pub what: String,
    pub msg: String,
}

impl std::fmt::Display for GrammarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.what, self.msg)
    }
}

/// An instance (lexical entry, rule, ...) as written in TDL.
#[derive(Debug, Clone)]
pub struct Instance {
    pub name: String,
    pub status: Option<String>,
    pub body: Conj,
    pub affix: Option<emdysi_tdl::Affix>,
    pub docstrings: Vec<String>,
}

pub struct Grammar {
    pub ts: TypeSystem,
    pub feats: Features,
    pub lists: ListConfig,
    /// For each feature, the most general type it is appropriate for.
    pub intro: Vec<TypeId>,
    /// Expanded constraint of every type, indexed by type id.
    constraints: Vec<Arc<Dag>>,
    /// Whether a type's constraint is just the type (no features).
    atomic: Vec<bool>,
    pub instances: Vec<Instance>,
    pub instance_index: HashMap<String, usize>,
    pub letter_sets: Vec<emdysi_tdl::LetterSet>,
    /// Problems found while compiling; the grammar is still usable.
    pub errors: Vec<GrammarError>,
}

impl Grammar {
    /// Compile everything loaded from a grammar's top-level TDL file.
    pub fn compile(loaded: &emdysi_tdl::Loaded) -> Result<Grammar, GrammarError> {
        let decls = crate::type_declarations(&loaded.entries);
        let hier = Hierarchy::build(&decls).map_err(|e| GrammarError {
            what: "hierarchy".into(),
            msg: e.to_string(),
        })?;
        let ts = TypeSystem::new(hier);
        let mut feats = Features::default();
        let lists = ListConfig {
            list: type_id(&ts, "*list*")?,
            cons: type_id(&ts, "*cons*")?,
            null: type_id(&ts, "*null*")?,
            diff_list: type_id(&ts, "*diff-list*")?,
            first: feats.intern("FIRST"),
            rest: feats.intern("REST"),
            list_feat: feats.intern("LIST"),
            last: feats.intern("LAST"),
        };

        // Local descriptions of types, merging addenda.
        let mut bodies: HashMap<TypeId, Vec<&Conj>> = HashMap::new();
        for e in loaded.entries.iter().filter(|e| e.env == Env::Type) {
            let t = ts.hier.id(&e.def.name).expect("declared");
            bodies.entry(t).or_default().push(&e.def.body);
        }
        let mut errors = Vec::new();
        let n = ts.hier.len();
        let mut descs: Vec<Arc<Dag>> = Vec::with_capacity(n);
        for t in 0..n as TypeId {
            let dag = match bodies.get(&t) {
                None => Dag::atomic(TOP),
                Some(conjs) => {
                    let b = DescBuilder::new(&ts, &mut feats, lists);
                    match b.build_all(conjs) {
                        Ok(d) => d,
                        Err(e) => {
                            errors.push(GrammarError {
                                what: ts.hier.name(t).into(),
                                msg: e.0,
                            });
                            Dag::atomic(TOP)
                        }
                    }
                }
            };
            descs.push(Arc::new(dag));
        }

        let intro = compute_intro(&ts, &feats, &descs, &mut errors);

        let mut g = Grammar {
            ts,
            feats,
            lists,
            intro,
            constraints: Vec::new(),
            atomic: Vec::new(),
            instances: Vec::new(),
            instance_index: HashMap::new(),
            letter_sets: loaded.letter_sets.clone(),
            errors,
        };
        g.expand_types(&descs);

        for e in &loaded.entries {
            if let Env::Instance(status) = &e.env {
                let name = e.def.name.to_lowercase();
                g.instance_index.insert(name.clone(), g.instances.len());
                g.instances.push(Instance {
                    name,
                    status: status.clone(),
                    body: e.def.body.clone(),
                    affix: e.def.affix.clone(),
                    docstrings: e.def.docstrings.clone(),
                });
            }
        }
        Ok(g)
    }

    pub fn constraint(&self, t: TypeId) -> &Arc<Dag> {
        &self.constraints[t as usize]
    }

    /// Constraint lookup in the form the unifier wants.
    pub fn constraint_fn(&self) -> impl Fn(TypeId) -> Result<Option<Arc<Dag>>, TypeId> + '_ {
        move |t| {
            if TypeSystem::is_literal(t) || self.atomic[t as usize] {
                Ok(None)
            } else {
                Ok(Some(self.constraints[t as usize].clone()))
            }
        }
    }

    pub fn feat(&self, name: &str) -> Option<FeatId> {
        self.feats.get(name)
    }

    pub fn path(&self, path: &str) -> Option<Vec<FeatId>> {
        path.split(['.', ' '])
            .filter(|s| !s.is_empty())
            .map(|f| self.feats.get(f))
            .collect()
    }

    pub fn instance(&self, name: &str) -> Option<&Instance> {
        self.instance_index
            .get(&name.to_lowercase())
            .map(|&i| &self.instances[i])
    }

    pub fn display(&self, dag: &Dag) -> String {
        dag.display(&self.ts, &self.feats)
    }

    fn expand_types(&mut self, descs: &[Arc<Dag>]) {
        let n = self.ts.hier.len();
        let mut state = Expansion {
            done: vec![None; n],
            on_stack: vec![false; n],
            unifier: Unifier::new(),
        };
        for t in self.ts.hier.topological() {
            self.ensure(t, descs, &mut state);
        }
        self.atomic = state
            .done
            .iter()
            .map(|d| d.as_ref().is_none_or(|d| d.arcs.is_empty()))
            .collect();
        self.constraints = state
            .done
            .into_iter()
            .enumerate()
            .map(|(t, d)| d.unwrap_or_else(|| Arc::new(Dag::atomic(t as TypeId))))
            .collect();
    }

    /// Types whose constraints must be known before `t` can be expanded.
    fn static_deps(&self, t: TypeId, desc: &Dag) -> Vec<TypeId> {
        let mut deps: Vec<TypeId> = self.ts.hier.parents(t).to_vec();
        for n in 1..desc.nodes.len() as u32 {
            if let Some(tt) = self.node_type_with_intro(desc, n) {
                if !TypeSystem::is_literal(tt) {
                    deps.push(tt);
                }
            }
        }
        deps.sort_unstable();
        deps.dedup();
        deps.retain(|&d| d != t);
        deps
    }

    /// The type of a description node narrowed by the types that introduce
    /// its features, or `None` if they clash.
    fn node_type_with_intro(&self, desc: &Dag, n: u32) -> Option<TypeId> {
        let mut t = desc.ty(n);
        for &(f, _) in desc.arcs(n) {
            t = self.ts.glb(t, self.intro[f as usize])?;
        }
        Some(t)
    }

    fn ensure(&mut self, t0: TypeId, descs: &[Arc<Dag>], st: &mut Expansion) {
        let mut stack = vec![t0];
        st.on_stack[t0 as usize] = true;
        while let Some(&t) = stack.last() {
            if st.done[t as usize].is_some() {
                st.on_stack[t as usize] = false;
                stack.pop();
                continue;
            }
            let pending: Vec<TypeId> = self
                .static_deps(t, &descs[t as usize])
                .into_iter()
                .filter(|&d| st.done[d as usize].is_none())
                .collect();
            if let Some(&d) = pending.iter().find(|&&d| st.on_stack[d as usize]) {
                self.fail(
                    t,
                    st,
                    format!("cyclic dependency on {}", self.ts.hier.name(d)),
                );
                continue;
            }
            // Push one dependency at a time so the stack is a dependency path
            // and `on_stack` identifies genuine cycles.
            if let Some(&d) = pending.first() {
                st.on_stack[d as usize] = true;
                stack.push(d);
                continue;
            }
            match self.try_expand(t, &descs[t as usize], st) {
                Ok(dag) => st.done[t as usize] = Some(Arc::new(dag)),
                Err(Failure::NeedConstraint(d)) if st.on_stack[d as usize] => {
                    let msg = format!(
                        "needs constraint of {} which depends on it",
                        self.ts.hier.name(d)
                    );
                    self.fail(t, st, msg);
                }
                Err(Failure::NeedConstraint(d)) => {
                    st.on_stack[d as usize] = true;
                    stack.push(d);
                }
                Err(Failure::Clash) => self.fail(t, st, "inconsistent constraint".into()),
            }
        }
    }

    fn fail(&mut self, t: TypeId, st: &mut Expansion, msg: String) {
        self.errors.push(GrammarError {
            what: self.ts.hier.name(t).into(),
            msg,
        });
        st.done[t as usize] = Some(Arc::new(Dag::atomic(t)));
    }

    fn try_expand(&self, t: TypeId, desc: &Arc<Dag>, st: &mut Expansion) -> Result<Dag, Failure> {
        let done = &st.done;
        let atomic_ok = |u: TypeId| -> Result<Option<Arc<Dag>>, TypeId> {
            if TypeSystem::is_literal(u) {
                return Ok(None);
            }
            match &done[u as usize] {
                Some(d) if d.arcs.is_empty() => Ok(None),
                Some(d) => Ok(Some(d.clone())),
                None => Err(u),
            }
        };
        let u = &mut st.unifier;
        u.begin();
        let root = u.add(desc.clone());
        // Fix the root type first: unifying in the parents' constraints must
        // not ask for the constraint of `t` itself.
        u.set_type(root, t);
        let fail = |u: &Unifier| u.failure.unwrap_or(Failure::Clash);
        for n in 1..desc.nodes.len() as u32 {
            let Some(tt) = self.node_type_with_intro(desc, n) else {
                return Err(Failure::Clash);
            };
            if TypeSystem::is_literal(tt) {
                continue;
            }
            match atomic_ok(tt).map_err(Failure::NeedConstraint)? {
                Some(c) => {
                    let r = u.add(c);
                    if !u.unify(root + n, r, &self.ts, &atomic_ok) {
                        return Err(fail(u));
                    }
                }
                None => {
                    let h = root + n;
                    if u.node_type(h) != tt {
                        let a = u.add(Arc::new(Dag::atomic(tt)));
                        if !u.unify(h, a, &self.ts, &atomic_ok) {
                            return Err(fail(u));
                        }
                    }
                }
            }
        }
        for &p in self.ts.hier.parents(t) {
            if let Some(c) = atomic_ok(p).map_err(Failure::NeedConstraint)? {
                let r = u.add(c);
                if !u.unify(root, r, &self.ts, &atomic_ok) {
                    return Err(fail(u));
                }
            }
        }
        u.copy(root, &[]).ok_or(Failure::Clash)
    }

    /// Expand an instance description (lexical entry, rule, ...) into a
    /// well-typed feature structure.
    pub fn expand(&self, body: &Conj, u: &mut Unifier) -> Result<Dag, String> {
        let mut feats = self.feats.clone();
        let desc = DescBuilder::new(&self.ts, &mut feats, self.lists)
            .build(body)
            .map_err(|e| e.0)?;
        if feats.len() != self.feats.len() {
            return Err("description uses an unknown feature".into());
        }
        self.expand_desc(Arc::new(desc), u)
    }

    pub fn expand_desc(&self, desc: Arc<Dag>, u: &mut Unifier) -> Result<Dag, String> {
        let cons = self.constraint_fn();
        u.begin();
        let root = u.add(desc.clone());
        for n in 0..desc.nodes.len() as u32 {
            let tt = self
                .node_type_with_intro(&desc, n)
                .ok_or("feature not appropriate for type")?;
            if TypeSystem::is_literal(tt) {
                continue;
            }
            let r = u.add(self.constraints[tt as usize].clone());
            if !u.unify(root + n, r, &self.ts, &cons) {
                return Err(format!("unification failed at node {n}"));
            }
        }
        u.copy(root, &[])
            .ok_or_else(|| "cyclic structure".to_string())
    }
}

struct Expansion {
    done: Vec<Option<Arc<Dag>>>,
    on_stack: Vec<bool>,
    unifier: Unifier,
}

fn type_id(ts: &TypeSystem, name: &str) -> Result<TypeId, GrammarError> {
    ts.hier.id(name).ok_or_else(|| GrammarError {
        what: name.into(),
        msg: "required type is not defined".into(),
    })
}

/// Find, for each feature, the most general type that declares it.
fn compute_intro(
    ts: &TypeSystem,
    feats: &Features,
    descs: &[Arc<Dag>],
    errors: &mut Vec<GrammarError>,
) -> Vec<TypeId> {
    let mut intro: Vec<Option<TypeId>> = vec![None; feats.len()];
    for t in ts.hier.topological() {
        for &(f, _) in descs[t as usize].arcs(0) {
            match intro[f as usize] {
                None => intro[f as usize] = Some(t),
                Some(i) if ts.hier.subsumed_by(t, i) => {}
                Some(i) => errors.push(GrammarError {
                    what: feats.name(f).into(),
                    msg: format!(
                        "feature introduced at unrelated types {} and {}",
                        ts.hier.name(i),
                        ts.hier.name(t)
                    ),
                }),
            }
        }
    }
    intro
        .into_iter()
        .enumerate()
        .map(|(f, t)| {
            t.unwrap_or_else(|| {
                errors.push(GrammarError {
                    what: feats.name(f as FeatId).into(),
                    msg: "feature is never introduced at the top level of a type".into(),
                });
                TOP
            })
        })
        .collect()
}
