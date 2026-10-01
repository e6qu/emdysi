//! Lexical lookup: matching lexical entries against a token lattice.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use emdysi_tdl::Term;

use crate::chartmap::{Lattice, string_at};
use crate::dag::{Dag, Node};
use crate::grammar::Grammar;
use crate::morph::Morphology;
use crate::typesys::FeatId;
use crate::unify::Unifier;

#[derive(Debug, Clone)]
pub struct LexEntry {
    /// Index into [`Grammar::instances`].
    pub inst: usize,
    /// Lower-cased orthography.
    pub orth: Vec<String>,
}

/// Paths into lexical entries and tokens.
#[derive(Debug, Clone)]
pub struct LexPaths {
    pub tokens_list: Vec<FeatId>,
    pub tokens_last: Vec<FeatId>,
    pub token_form: Vec<FeatId>,
}

pub struct Lexicon {
    pub entries: Vec<LexEntry>,
    by_first: HashMap<String, Vec<usize>>,
    /// Last words of all entries: the forms morphology may produce.
    stems: HashSet<String>,
    pub generics: Vec<usize>,
    pub morph: Morphology,
    pub paths: LexPaths,
    cache: Mutex<HashMap<usize, Arc<Dag>>>,
}

/// A lexical entry instantiated over a sequence of tokens.
#[derive(Debug, Clone)]
pub struct LexItem {
    /// Instance index of the entry.
    pub inst: usize,
    /// Lattice entries covered, in order.
    pub tokens: Vec<usize>,
    pub start: u32,
    pub end: u32,
    pub dag: Arc<Dag>,
    /// Orthographic rules still to apply, innermost first.
    pub pending: Vec<String>,
}

fn orth_of(body: &emdysi_tdl::Conj) -> Option<Vec<String>> {
    for term in &body.0 {
        if let Term::Avm(fvs) = term {
            for fv in fvs {
                if fv.path.len() == 1 && fv.path[0].eq_ignore_ascii_case("ORTH") {
                    for t in &fv.value.0 {
                        if let Term::List { items, .. } = t {
                            let words: Option<Vec<String>> = items
                                .iter()
                                .map(|c| match c.0.first() {
                                    Some(Term::Str(s)) => Some(s.to_lowercase()),
                                    _ => None,
                                })
                                .collect();
                            return words;
                        }
                    }
                }
            }
        }
    }
    None
}

impl Lexicon {
    pub fn new(g: &Grammar, morph: Morphology, paths: LexPaths) -> Lexicon {
        let mut entries = Vec::new();
        let mut by_first: HashMap<String, Vec<usize>> = HashMap::new();
        let mut stems = HashSet::new();
        let mut generics = Vec::new();
        for (i, inst) in g.instances.iter().enumerate() {
            match inst.status.as_deref() {
                Some("lex-entry") => {
                    if let Some(orth) = orth_of(&inst.body).filter(|o| !o.is_empty()) {
                        by_first
                            .entry(orth[0].clone())
                            .or_default()
                            .push(entries.len());
                        stems.insert(orth.last().unwrap().clone());
                        entries.push(LexEntry { inst: i, orth });
                    }
                }
                Some("generic-lex-entry") => generics.push(i),
                _ => {}
            }
        }
        Lexicon {
            entries,
            by_first,
            stems,
            generics,
            morph,
            paths,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// The expanded feature structure of an instance, cached.
    pub fn entry_dag(&self, g: &Grammar, inst: usize, u: &mut Unifier) -> Option<Arc<Dag>> {
        if let Some(d) = self.cache.lock().unwrap().get(&inst) {
            return Some(d.clone());
        }
        let d = Arc::new(g.expand(&g.instances[inst].body, u).ok()?);
        self.cache.lock().unwrap().insert(inst, d.clone());
        Some(d)
    }

    /// Whether some entry's orthography starts with this word.
    pub fn has_first_word(&self, w: &str) -> bool {
        self.by_first.contains_key(w)
    }

    pub fn is_stem(&self, s: &str) -> bool {
        self.stems.contains(s)
    }

    /// Instantiate all native and generic lexical entries over the lattice.
    pub fn instantiate(&self, g: &Grammar, lat: &Lattice, u: &mut Unifier) -> Vec<LexItem> {
        let mut out = Vec::new();
        let alive: Vec<(usize, &crate::chartmap::Entry)> = lat.alive().collect();
        let forms: HashMap<usize, String> = alive
            .iter()
            .filter_map(|(id, e)| {
                string_at(&g.ts, &e.dag, &self.paths.token_form).map(|f| (*id, f.to_lowercase()))
            })
            .collect();
        let mut analyses: HashMap<String, Vec<crate::morph::Analysis>> = HashMap::new();
        for (id, e) in &alive {
            let Some(form) = forms.get(id) else { continue };
            // Native entries starting at this token.
            if let Some(cands) = self.by_first.get(form) {
                for &ei in cands {
                    let entry = &self.entries[ei];
                    if entry.orth.len() == 1 {
                        out.extend(self.make(g, entry.inst, &[*id], lat, Vec::new(), u));
                    } else {
                        self.multiword(
                            g,
                            entry,
                            *id,
                            e.end,
                            1,
                            vec![*id],
                            &forms,
                            lat,
                            &mut analyses,
                            u,
                            &mut out,
                        );
                    }
                }
            }
            // Inflected single-word entries.
            let an = analyses
                .entry(form.clone())
                .or_insert_with(|| self.morph.analyze(form, &|s| self.is_stem(s), 3))
                .clone();
            for a in an.iter().filter(|a| !a.rules.is_empty()) {
                if let Some(cands) = self.by_first.get(&a.stem) {
                    for &ei in cands {
                        if self.entries[ei].orth.len() == 1 {
                            out.extend(self.make(
                                g,
                                self.entries[ei].inst,
                                &[*id],
                                lat,
                                a.rules.clone(),
                                u,
                            ));
                        }
                    }
                }
            }
            // Generic entries; unification with the token decides.
            for &gi in &self.generics {
                out.extend(self.make(g, gi, &[*id], lat, Vec::new(), u));
            }
        }
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn multiword(
        &self,
        g: &Grammar,
        entry: &LexEntry,
        _first: usize,
        at: u32,
        k: usize,
        so_far: Vec<usize>,
        forms: &HashMap<usize, String>,
        lat: &Lattice,
        analyses: &mut HashMap<String, Vec<crate::morph::Analysis>>,
        u: &mut Unifier,
        out: &mut Vec<LexItem>,
    ) {
        let last = k + 1 == entry.orth.len();
        let next: Vec<(usize, u32)> = lat
            .alive()
            .filter(|(_, e)| e.start == at)
            .map(|(i, e)| (i, e.end))
            .collect();
        for (id, end) in next {
            let Some(form) = forms.get(&id) else { continue };
            let mut toks = so_far.clone();
            toks.push(id);
            if *form == entry.orth[k] {
                if last {
                    out.extend(self.make(g, entry.inst, &toks, lat, Vec::new(), u));
                } else {
                    self.multiword(
                        g,
                        entry,
                        _first,
                        end,
                        k + 1,
                        toks.clone(),
                        forms,
                        lat,
                        analyses,
                        u,
                        out,
                    );
                }
            }
            if last {
                // The last word may be inflected.
                let an = analyses
                    .entry(form.clone())
                    .or_insert_with(|| self.morph.analyze(form, &|s| self.is_stem(s), 3))
                    .clone();
                for a in an
                    .iter()
                    .filter(|a| !a.rules.is_empty() && a.stem == entry.orth[k])
                {
                    out.extend(self.make(g, entry.inst, &toks, lat, a.rules.clone(), u));
                }
            }
        }
    }

    /// Unify an entry with the tokens it covers.
    fn make(
        &self,
        g: &Grammar,
        inst: usize,
        toks: &[usize],
        lat: &Lattice,
        pending: Vec<String>,
        u: &mut Unifier,
    ) -> Option<LexItem> {
        let entry = self.entry_dag(g, inst, u)?;
        let token_dags: Vec<&Dag> = toks.iter().map(|&t| &*lat.entries[t].dag).collect();
        let (list, roots) = list_dag(g, &token_dags);
        let cons = g.constraint_fn();
        u.begin();
        let r = u.add(entry);
        let l = u.add(Arc::new(list));
        let list_slot = u.follow(r, &self.paths.tokens_list)?;
        if !u.unify(list_slot, l, &g.ts, &cons) {
            return None;
        }
        let last_slot = u.follow(r, &self.paths.tokens_last)?;
        if !u.unify(last_slot, l + *roots.last()?, &g.ts, &cons) {
            return None;
        }
        let dag = u.copy(r, &[])?;
        Some(LexItem {
            inst,
            tokens: toks.to_vec(),
            start: lat.entries[toks[0]].start,
            end: lat.entries[*toks.last()?].end,
            dag: Arc::new(dag),
            pending,
        })
    }
}

/// Build a closed list whose elements are the given DAGs. Returns the list
/// and the node index of each element's root.
pub fn list_dag(g: &Grammar, items: &[&Dag]) -> (Dag, Vec<u32>) {
    let mut dag = Dag {
        nodes: Vec::new(),
        arcs: Vec::new(),
    };
    // Cons cells first, then each item's nodes appended with an offset.
    let n = items.len();
    let cells = n + 1;
    let mut offsets = Vec::with_capacity(n);
    let mut next = cells as u32;
    for it in items {
        offsets.push(next);
        next += it.nodes.len() as u32;
    }
    let (first, rest) = (g.lists.first, g.lists.rest);
    for (i, &off) in offsets.iter().enumerate() {
        let start = dag.arcs.len() as u32;
        let mut arcs = [(first, off), (rest, i as u32 + 1)];
        arcs.sort_by_key(|a| a.0);
        dag.arcs.extend(arcs);
        dag.nodes.push(Node {
            ty: g.lists.cons,
            arc_start: start,
            arc_len: 2,
        });
    }
    dag.nodes.push(Node {
        ty: g.lists.null,
        arc_start: dag.arcs.len() as u32,
        arc_len: 0,
    });
    for (it, &off) in items.iter().zip(&offsets) {
        let arc_base = dag.arcs.len() as u32;
        for node in &it.nodes {
            dag.nodes.push(Node {
                ty: node.ty,
                arc_start: node.arc_start + arc_base,
                arc_len: node.arc_len,
            });
        }
        dag.arcs.extend(it.arcs.iter().map(|&(f, v)| (f, v + off)));
    }
    (dag, offsets)
}
