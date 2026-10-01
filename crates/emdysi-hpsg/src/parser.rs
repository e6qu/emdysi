//! Bottom-up chart parsing with typed feature structure grammars.
//!
//! Parsing runs in three phases, as in ACE: lexical rules are applied to the
//! instantiated lexical entries (orthographic rules only in the order the
//! morphological analysis prescribes), the resulting lexical edges are
//! filtered with lexical-filtering chart-mapping rules, and then syntactic
//! rules are applied exhaustively with an agenda.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::chartmap::{ChartMapper, Lattice, MapRule};
use crate::dag::{Dag, Subsumer};
use crate::grammar::Grammar;
use crate::lexicon::LexItem;
use crate::types::{TOP, TypeId};
use crate::typesys::FeatId;
use crate::unify::Unifier;

/// Quick-check: types at a few paths that most often cause unification
/// failures, compared before attempting a full unification.
#[derive(Debug, Clone, Default)]
pub struct QuickCheck {
    pub paths: Vec<Vec<FeatId>>,
}

impl QuickCheck {
    /// Read an ACE quick-check file (`PUSH(F)`, `POP`, `REC(n)` operations).
    pub fn parse_ace(g: &Grammar, src: &str) -> QuickCheck {
        let mut slots: Vec<(usize, Vec<FeatId>)> = Vec::new();
        let mut stack: Vec<Option<FeatId>> = Vec::new();
        for tok in src.split_whitespace() {
            if let Some(f) = tok.strip_prefix("PUSH(").and_then(|t| t.strip_suffix(')')) {
                stack.push(g.feat(f));
            } else if tok == "POP" {
                stack.pop();
            } else if let Some(n) = tok.strip_prefix("REC(").and_then(|t| t.strip_suffix(')')) {
                if let (Ok(n), Some(path)) = (
                    n.parse::<usize>(),
                    stack.iter().copied().collect::<Option<Vec<_>>>(),
                ) {
                    slots.push((n, path));
                }
            }
        }
        slots.sort_by_key(|s| s.0);
        QuickCheck {
            paths: slots.into_iter().map(|s| s.1).collect(),
        }
    }

    pub fn vector(&self, dag: &Dag, node: u32) -> Vec<TypeId> {
        self.paths
            .iter()
            .map(|p| dag.follow(node, p).map_or(TOP, |n| dag.ty(n)))
            .collect()
    }

    pub fn compatible(g: &Grammar, a: &[TypeId], b: &[TypeId]) -> bool {
        a.iter()
            .zip(b)
            .all(|(&x, &y)| x == TOP || y == TOP || g.ts.glb(x, y).is_some())
    }
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub name: String,
    pub dag: Arc<Dag>,
    /// Node index of each daughter in `dag`.
    pub dtrs: Vec<u32>,
    pub lexical: bool,
    /// Has orthographic (affixation) effect.
    pub orth: bool,
    pub spanning_only: bool,
    dtr_qc: Vec<Vec<TypeId>>,
}

impl Rule {
    pub fn new(
        g: &Grammar,
        name: &str,
        dag: Arc<Dag>,
        lexical: bool,
        orth: bool,
        qc: &QuickCheck,
        args: FeatId,
    ) -> Rule {
        let mut dtrs = Vec::new();
        if let Some(mut n) = dag.arc(0, args) {
            while let Some(first) = dag.arc(n, g.lists.first) {
                dtrs.push(first);
                match dag.arc(n, g.lists.rest) {
                    Some(r) => n = r,
                    None => break,
                }
            }
        }
        let dtr_qc = dtrs.iter().map(|&d| qc.vector(&dag, d)).collect();
        Rule {
            name: name.to_string(),
            dag,
            dtrs,
            lexical,
            orth,
            spanning_only: false,
            dtr_qc,
        }
    }
}

#[derive(Debug, Clone)]
pub enum EdgeKind {
    Lex { inst: usize, tokens: Vec<usize> },
    Rule(usize),
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub start: usize,
    pub end: usize,
    pub dag: Arc<Dag>,
    pub kind: EdgeKind,
    pub daughters: Vec<usize>,
    /// Orthographic rules still to apply (lexical edges only).
    pub pending: Vec<String>,
    pub lexical: bool,
    qc: Vec<TypeId>,
    /// For daughter positions 0 and 1, the rules whose daughter this edge
    /// passes the quick check for (bit per rule; syntactic phase only).
    fits: [Vec<u64>; 2],
    pub state: EdgeState,
    /// Edges packed into this one: equivalent or more specific analyses
    /// of the same span, recovered when unpacking.
    pub packed: Vec<usize>,
    /// Edges built with this edge as a daughter.
    parents: Vec<usize>,
    restricted: Option<Arc<Dag>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeState {
    Active,
    /// Packed into a more general edge.
    Packed(usize),
    /// Invalidated by retroactive packing.
    Frozen,
}

/// A node of a derivation tree.
#[derive(Debug, Clone)]
pub struct Deriv {
    pub kind: EdgeKind,
    pub start: usize,
    pub end: usize,
    pub daughters: Vec<Arc<Deriv>>,
    /// The feature structure of this node.
    pub dag: Arc<Dag>,
}

/// A complete analysis of the input.
#[derive(Debug, Clone)]
pub struct Reading {
    pub root: String,
    pub deriv: Arc<Deriv>,
    pub dag: Arc<Dag>,
}

#[derive(Clone)]
struct Inst {
    dag: Arc<Dag>,
    deriv: Arc<Deriv>,
}

fn bit(set: &[u64], i: usize) -> bool {
    set.get(i / 64).is_some_and(|w| w & (1 << (i % 64)) != 0)
}

pub struct ParserConfig {
    pub deleted_daughters: Vec<FeatId>,
    pub roots: Vec<(String, Arc<Dag>)>,
    pub max_edges: usize,
    pub timeout: Duration,
    /// Features ignored when comparing edges for ambiguity packing; `None`
    /// disables packing.
    pub packing_restrictor: Option<Vec<FeatId>>,
    /// Upper bound on readings recovered by unpacking.
    pub max_readings: usize,
    /// When packing, compare edges as if their top type were this one
    /// (ACE's `generalize-edge-top-types`, e.g. `sign` for the ERG).
    pub packing_top_type: Option<TypeId>,
}

/// Counters describing the work a parse did.
#[derive(Debug, Clone, Default)]
pub struct Stats {
    pub attempts: usize,
    pub qc_filtered: usize,
    pub unify_failed: usize,
    pub cyclic: usize,
    pub unify_time: Duration,
    pub copy_time: Duration,
    pub pack_time: Duration,
    pub fits_time: Duration,
    pub unpack_time: Duration,
    pub lexical_time: Duration,
    pub subsumption_checks: usize,
    pub restrict_time: Duration,
    pub packed_proactive: usize,
    pub packed_retroactive: usize,
    pub frozen: usize,
}

pub struct ParseResult {
    pub stats: Stats,
    pub chart: Vec<Edge>,
    pub readings: Vec<Reading>,
    pub positions: usize,
    pub exhausted: bool,
    pub filtered_lexical: usize,
}

/// A daughter quick-check vector and the (rule, daughter) slots using it.
type QcGroup = (Vec<TypeId>, Vec<(usize, usize)>);

pub struct Parser<'g> {
    pub g: &'g Grammar,
    pub rules: &'g [Rule],
    pub qc: &'g QuickCheck,
    pub config: &'g ParserConfig,
    pub lexical_filter: &'g [MapRule],
    /// Distinct daughter quick-check vectors of syntactic rules, with the
    /// (rule, daughter position) slots that use each.
    qc_groups: Vec<QcGroup>,
    /// Quick-check slots that pass through features ignored by packing.
    qc_restricted: Vec<bool>,
    subsumer: Subsumer,
    u: Unifier,
    chart: Vec<Edge>,
    by_start: Vec<Vec<usize>>,
    by_end: Vec<Vec<usize>>,
    n: usize,
    deadline: Instant,
    stats: Stats,
}

impl<'g> Parser<'g> {
    pub fn new(
        g: &'g Grammar,
        rules: &'g [Rule],
        qc: &'g QuickCheck,
        config: &'g ParserConfig,
        lexical_filter: &'g [MapRule],
    ) -> Self {
        let mut groups: HashMap<Vec<TypeId>, Vec<(usize, usize)>> = HashMap::new();
        for (ri, rule) in rules.iter().enumerate() {
            if rule.lexical {
                continue;
            }
            for (pos, dqc) in rule.dtr_qc.iter().enumerate().take(2) {
                groups.entry(dqc.clone()).or_default().push((ri, pos));
            }
        }
        let restrictor = config.packing_restrictor.clone().unwrap_or_default();
        let qc_restricted = qc
            .paths
            .iter()
            .map(|p| p.iter().any(|f| restrictor.contains(f)))
            .collect();
        Parser {
            g,
            rules,
            qc,
            config,
            lexical_filter,
            qc_groups: groups.into_iter().collect(),
            qc_restricted,
            subsumer: Subsumer::default(),
            u: Unifier::new(),
            chart: Vec::new(),
            by_start: Vec::new(),
            by_end: Vec::new(),
            n: 0,
            deadline: Instant::now(),
            stats: Stats::default(),
        }
    }

    pub fn parse(mut self, lat: &Lattice, items: Vec<LexItem>) -> ParseResult {
        self.deadline = Instant::now() + self.config.timeout;
        // Map lattice vertices to chart positions.
        let used: HashSet<u32> = items.iter().flat_map(|i| [i.start, i.end]).collect();
        let order: Vec<u32> = lat
            .ordered_vertices()
            .into_iter()
            .filter(|v| used.contains(v))
            .collect();
        let pos: HashMap<u32, usize> = order.iter().enumerate().map(|(i, &v)| (v, i)).collect();
        self.n = order.len().saturating_sub(1);
        self.by_start = vec![Vec::new(); self.n + 1];
        self.by_end = vec![Vec::new(); self.n + 1];

        // Phase 1: lexical rules.
        let mut lexical = Vec::new();
        let mut agenda: Vec<Edge> = items
            .into_iter()
            .map(|it| {
                let qc = self.qc.vector(&it.dag, 0);
                Edge {
                    start: pos[&it.start],
                    end: pos[&it.end],
                    dag: it.dag,
                    kind: EdgeKind::Lex {
                        inst: it.inst,
                        tokens: it.tokens,
                    },
                    daughters: Vec::new(),
                    pending: it.pending,
                    lexical: true,
                    qc,
                    fits: Default::default(),
                    state: EdgeState::Active,
                    packed: Vec::new(),
                    parents: Vec::new(),
                    restricted: None,
                }
            })
            .collect();
        let mut exhausted = false;
        let mut lex_edges: Vec<Edge> = Vec::new();
        while let Some(e) = agenda.pop() {
            if lex_edges.len() > self.config.max_edges || Instant::now() > self.deadline {
                exhausted = true;
                break;
            }
            let id = lex_edges.len();
            lex_edges.push(e);
            let e = &lex_edges[id];
            let mut new = Vec::new();
            for (ri, rule) in self.rules.iter().enumerate() {
                if !rule.lexical || rule.dtrs.len() != 1 {
                    continue;
                }
                let pending = if rule.orth {
                    match e.pending.first() {
                        Some(next) if *next == rule.name => e.pending[1..].to_vec(),
                        _ => continue,
                    }
                } else {
                    e.pending.clone()
                };
                if let Some(dag) = self.apply(ri, &[&e.dag], &[&e.qc]) {
                    new.push(Edge {
                        start: e.start,
                        end: e.end,
                        qc: self.qc.vector(&dag, 0),
                        dag: Arc::new(dag),
                        kind: EdgeKind::Rule(ri),
                        daughters: vec![id],
                        pending,
                        lexical: true,
                        fits: Default::default(),
                        state: EdgeState::Active,
                        packed: Vec::new(),
                        parents: Vec::new(),
                        restricted: None,
                    });
                }
            }
            agenda.extend(new);
        }
        // Lexical edges keep their index: the chart starts with all of them.
        for e in lex_edges {
            lexical.push(self.chart.len());
            self.chart.push(e);
        }

        // Phase 2: lexical filtering of complete lexical edges.
        let complete: Vec<usize> = lexical
            .iter()
            .copied()
            .filter(|&i| self.chart[i].pending.is_empty())
            .collect();
        let mut keep: HashSet<usize> = complete.iter().copied().collect();
        let mut filtered = 0;
        if !self.lexical_filter.is_empty() {
            let mut flat = Lattice::default();
            for v in 0..=self.n {
                flat.add_vertex(v as f64);
            }
            let ids: Vec<usize> = complete.clone();
            for &i in &ids {
                let e = &self.chart[i];
                flat.add(e.dag.clone(), e.start as u32, e.end as u32);
            }
            let mut mapper = ChartMapper::new(self.g, self.lexical_filter);
            mapper.apply(&mut flat);
            for (k, &i) in ids.iter().enumerate() {
                if !flat.entries[k].alive {
                    keep.remove(&i);
                    filtered += 1;
                }
            }
        }

        self.stats.lexical_time = Instant::now() - (self.deadline - self.config.timeout);
        // Phase 3: syntactic rules.
        let mut agenda: Vec<usize> = complete.into_iter().filter(|i| keep.contains(i)).collect();
        agenda.sort_unstable_by(|a, b| b.cmp(a));
        let mut by_span: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
        while let Some(id) = agenda.pop() {
            if self.chart.len() > self.config.max_edges || Instant::now() > self.deadline {
                exhausted = true;
                break;
            }
            if self.chart[id].state != EdgeState::Active {
                continue;
            }
            let (start, end) = (self.chart[id].start, self.chart[id].end);
            let t = Instant::now();
            let packed =
                self.config.packing_restrictor.is_some() && self.pack(id, &by_span, &mut agenda);
            self.stats.pack_time += t.elapsed();
            if packed {
                continue;
            }
            by_span.entry((start, end)).or_default().push(id);
            let t = Instant::now();
            self.compute_fits(id);
            self.stats.fits_time += t.elapsed();
            self.by_start[start].push(id);
            self.by_end[end].push(id);
            let active = |p: &Parser, e: usize| p.chart[e].state == EdgeState::Active;
            for ri in 0..self.rules.len() {
                let rule = &self.rules[ri];
                if rule.lexical {
                    continue;
                }
                match rule.dtrs.len() {
                    1 => {
                        if bit(&self.chart[id].fits[0], ri) {
                            self.try_rule(ri, &[id], &mut agenda);
                        }
                    }
                    2 => {
                        for pos in 0..2 {
                            if !bit(&self.chart[id].fits[pos], ri)
                                || self.chart[id].state != EdgeState::Active
                            {
                                continue;
                            }
                            let other = 1 - pos;
                            let cands: Vec<usize> = if pos == 0 {
                                &self.by_start[end]
                            } else {
                                &self.by_end[start]
                            }
                            .iter()
                            .copied()
                            .filter(|&c| {
                                c != id && active(&self, c) && bit(&self.chart[c].fits[other], ri)
                            })
                            .collect();
                            if cands.is_empty() {
                                continue;
                            }
                            // Unify the rule with this daughter once, then
                            // quick-check the other daughters against the
                            // combined structure.
                            let Some(qc_other) = self.partial_qc(ri, pos, id) else {
                                continue;
                            };
                            for c in cands {
                                if self.chart[id].state != EdgeState::Active {
                                    break;
                                }
                                if !QuickCheck::compatible(self.g, &qc_other, &self.chart[c].qc) {
                                    self.stats.qc_filtered += 1;
                                    continue;
                                }
                                let pair = if pos == 0 { [id, c] } else { [c, id] };
                                self.try_rule(ri, &pair, &mut agenda);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Recover readings from spanning edges.
        let spanning: Vec<usize> = self
            .by_start
            .first()
            .map(|v| {
                v.iter()
                    .copied()
                    .filter(|&i| {
                        self.chart[i].end == self.n && self.chart[i].state == EdgeState::Active
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut readings = Vec::new();
        let unpack_start = Instant::now();
        let mut memo: HashMap<usize, Vec<Inst>> = HashMap::new();
        'outer: for id in spanning {
            for inst in self.unpack(id, &mut memo) {
                if readings.len() >= self.config.max_readings || Instant::now() > self.deadline {
                    break 'outer;
                }
                for (name, root) in &self.config.roots {
                    if self.unifies(root, &inst.dag) {
                        readings.push(Reading {
                            root: name.clone(),
                            deriv: inst.deriv.clone(),
                            dag: inst.dag.clone(),
                        });
                        break;
                    }
                }
            }
        }
        self.stats.unpack_time = unpack_start.elapsed();
        ParseResult {
            stats: self.stats,
            chart: self.chart,
            readings,
            positions: self.n,
            exhausted,
            filtered_lexical: filtered,
        }
    }

    /// Ambiguity packing (Oepen & Carroll 2000). Returns true if the edge
    /// was packed into an existing, more general edge.
    fn pack(
        &mut self,
        id: usize,
        by_span: &HashMap<(usize, usize), Vec<usize>>,
        agenda: &mut Vec<usize>,
    ) -> bool {
        let restrictor = self.config.packing_restrictor.as_deref().unwrap_or(&[]);
        let t = Instant::now();
        let mut r = self.chart[id].dag.restrict(restrictor);
        if let Some(top) = self.config.packing_top_type {
            r.nodes[0].ty = top;
        }
        let r = Arc::new(r);
        self.stats.restrict_time += t.elapsed();
        self.chart[id].restricted = Some(r.clone());
        let key = (self.chart[id].start, self.chart[id].end);
        let Some(others) = by_span.get(&key) else {
            return false;
        };
        for &o in others {
            if self.chart[o].state != EdgeState::Active {
                continue;
            }
            let (qf, qb) = self.qc_subsumption(&self.chart[o].qc, &self.chart[id].qc);
            if !qf && !qb {
                continue;
            }
            let Some(or) = self.chart[o].restricted.clone() else {
                continue;
            };
            self.stats.subsumption_checks += 1;
            let (o_subsumes, id_subsumes) = self.subsumer.check(&self.g.ts, &or, &r);
            if o_subsumes {
                self.chart[id].state = EdgeState::Packed(o);
                self.chart[o].packed.push(id);
                self.stats.packed_proactive += 1;
                return true;
            }
            if id_subsumes {
                self.chart[o].state = EdgeState::Packed(id);
                self.chart[id].packed.push(o);
                self.stats.packed_retroactive += 1;
                self.freeze_parents(o, agenda);
            }
        }
        false
    }

    /// Invalidate everything built from an edge that was packed
    /// retroactively; edges packed into invalidated ones get a second chance.
    fn freeze_parents(&mut self, id: usize, agenda: &mut Vec<usize>) {
        let mut stack: Vec<usize> = self.chart[id].parents.clone();
        while let Some(p) = stack.pop() {
            match self.chart[p].state {
                EdgeState::Frozen => continue,
                EdgeState::Packed(host) => self.chart[host].packed.retain(|&x| x != p),
                EdgeState::Active => {}
            }
            self.chart[p].state = EdgeState::Frozen;
            self.stats.frozen += 1;
            stack.extend(self.chart[p].parents.iter().copied());
            for q in std::mem::take(&mut self.chart[p].packed) {
                self.chart[q].state = EdgeState::Active;
                agenda.push(q);
            }
        }
    }

    /// All instantiations of an edge and the edges packed into it.
    fn unpack(&mut self, id: usize, memo: &mut HashMap<usize, Vec<Inst>>) -> Vec<Inst> {
        const MAX_PER_EDGE: usize = 1000;
        if let Some(v) = memo.get(&id) {
            return v.clone();
        }
        let mut alts = vec![id];
        let mut i = 0;
        while i < alts.len() {
            for &p in &self.chart[alts[i]].packed {
                if !alts.contains(&p) {
                    alts.push(p);
                }
            }
            i += 1;
        }
        let mut out = Vec::new();
        for a in alts {
            if out.len() >= MAX_PER_EDGE || Instant::now() > self.deadline {
                break;
            }
            let e = &self.chart[a];
            if e.lexical {
                out.push(Inst {
                    dag: e.dag.clone(),
                    deriv: Arc::new(self.chart_deriv(a)),
                });
                continue;
            }
            let EdgeKind::Rule(ri) = e.kind.clone() else {
                continue;
            };
            let (start, end) = (e.start, e.end);
            let daughters = e.daughters.clone();
            let kid_insts: Vec<Vec<Inst>> =
                daughters.iter().map(|&d| self.unpack(d, memo)).collect();
            // Cartesian product of daughter instantiations.
            let mut combos: Vec<Vec<usize>> = vec![Vec::new()];
            for k in &kid_insts {
                let mut next = Vec::new();
                for c in &combos {
                    for j in 0..k.len() {
                        let mut c2 = c.clone();
                        c2.push(j);
                        next.push(c2);
                    }
                }
                combos = next;
                if combos.len() > MAX_PER_EDGE {
                    combos.truncate(MAX_PER_EDGE);
                }
            }
            for c in combos {
                let dags: Vec<&Arc<Dag>> = c
                    .iter()
                    .enumerate()
                    .map(|(k, &j)| &kid_insts[k][j].dag)
                    .collect();
                let dags: Vec<Arc<Dag>> = dags.into_iter().cloned().collect();
                let dag_refs: Vec<&Arc<Dag>> = dags.iter().collect();
                let qcs: Vec<Vec<TypeId>> = dags.iter().map(|d| self.qc.vector(d, 0)).collect();
                let qc_refs: Vec<&Vec<TypeId>> = qcs.iter().collect();
                if let Some(m) = self.apply(ri, &dag_refs, &qc_refs) {
                    let m = Arc::new(m);
                    out.push(Inst {
                        dag: m.clone(),
                        deriv: Arc::new(Deriv {
                            kind: EdgeKind::Rule(ri),
                            start,
                            end,
                            daughters: c
                                .iter()
                                .enumerate()
                                .map(|(k, &j)| kid_insts[k][j].deriv.clone())
                                .collect(),
                            dag: m,
                        }),
                    });
                }
            }
        }
        memo.insert(id, out.clone());
        out
    }

    fn chart_deriv(&self, id: usize) -> Deriv {
        let e = &self.chart[id];
        Deriv {
            kind: e.kind.clone(),
            start: e.start,
            end: e.end,
            daughters: e
                .daughters
                .iter()
                .map(|&d| Arc::new(self.chart_deriv(d)))
                .collect(),
            dag: e.dag.clone(),
        }
    }

    /// Record which syntactic rule daughters an edge passes the quick check for.
    fn compute_fits(&mut self, id: usize) {
        let words = self.rules.len().div_ceil(64);
        let mut fits = [vec![0u64; words], vec![0u64; words]];
        let qc = &self.chart[id].qc;
        for (dqc, slots) in &self.qc_groups {
            if QuickCheck::compatible(self.g, dqc, qc) {
                for &(ri, pos) in slots {
                    fits[pos][ri / 64] |= 1 << (ri % 64);
                }
            }
        }
        self.chart[id].fits = fits;
    }

    /// Unify rule `ri` with edge `id` as daughter `pos` and return the
    /// quick-check vector of the other daughter slot, or `None` if the
    /// edge does not fit the rule at all.
    fn partial_qc(&mut self, ri: usize, pos: usize, id: usize) -> Option<Vec<TypeId>> {
        let rule = &self.rules[ri];
        let cons = self.g.constraint_fn();
        let t = Instant::now();
        self.u.begin();
        let r = self.u.add(rule.dag.clone());
        let h = self.u.add(self.chart[id].dag.clone());
        let ok = self.u.unify(r + rule.dtrs[pos], h, &self.g.ts, &cons);
        self.stats.unify_time += t.elapsed();
        if !ok {
            self.stats.unify_failed += 1;
            return None;
        }
        let slot = r + rule.dtrs[1 - pos];
        Some(
            self.qc
                .paths
                .iter()
                .map(|p| self.u.follow(slot, p).map_or(TOP, |n| self.u.node_type(n)))
                .collect(),
        )
    }

    /// Whether quick-check vectors allow `a` to subsume `b` and vice versa,
    /// ignoring slots under restricted features.
    fn qc_subsumption(&self, a: &[TypeId], b: &[TypeId]) -> (bool, bool) {
        let (mut fwd, mut bwd) = (true, true);
        for (i, (&x, &y)) in a.iter().zip(b).enumerate() {
            if x == y || self.qc_restricted[i] {
                continue;
            }
            if !self.g.ts.subsumed_by(y, x) {
                fwd = false;
            }
            if !self.g.ts.subsumed_by(x, y) {
                bwd = false;
            }
            if !fwd && !bwd {
                break;
            }
        }
        (fwd, bwd)
    }

    fn unifies(&mut self, a: &Arc<Dag>, b: &Arc<Dag>) -> bool {
        let cons = self.g.constraint_fn();
        self.u.begin();
        let x = self.u.add(a.clone());
        let y = self.u.add(b.clone());
        self.u.unify(x, y, &self.g.ts, &cons)
    }

    fn try_rule(&mut self, ri: usize, dtrs: &[usize], agenda: &mut Vec<usize>) {
        let rule = &self.rules[ri];
        let start = self.chart[dtrs[0]].start;
        let end = self.chart[*dtrs.last().unwrap()].end;
        if rule.spanning_only && !(start == 0 && end == self.n) {
            return;
        }
        // The quick check was done when the edges were indexed (`fits`).
        self.stats.attempts += 1;
        let dags: Vec<Arc<Dag>> = dtrs.iter().map(|&d| self.chart[d].dag.clone()).collect();
        if let Some(dag) = self.unify_rule(ri, &dags) {
            let id = self.chart.len();
            self.chart.push(Edge {
                start,
                end,
                qc: self.qc.vector(&dag, 0),
                dag: Arc::new(dag),
                kind: EdgeKind::Rule(ri),
                daughters: dtrs.to_vec(),
                pending: Vec::new(),
                lexical: false,
                fits: Default::default(),
                state: EdgeState::Active,
                packed: Vec::new(),
                parents: Vec::new(),
                restricted: None,
            });
            for &d in dtrs {
                self.chart[d].parents.push(id);
            }
            agenda.push(id);
        }
    }

    /// Unify a rule with its daughters and copy out the mother, after a
    /// quick check of each daughter.
    fn apply(&mut self, ri: usize, dtrs: &[&Arc<Dag>], qcs: &[&Vec<TypeId>]) -> Option<Dag> {
        self.stats.attempts += 1;
        let rule = &self.rules[ri];
        for (i, qc) in qcs.iter().enumerate() {
            if !QuickCheck::compatible(self.g, &rule.dtr_qc[i], qc) {
                self.stats.qc_filtered += 1;
                return None;
            }
        }
        let dags: Vec<Arc<Dag>> = dtrs.iter().map(|d| (*d).clone()).collect();
        self.unify_rule(ri, &dags)
    }

    /// Unify a rule with its daughters (already quick-checked).
    fn unify_rule(&mut self, ri: usize, dtrs: &[Arc<Dag>]) -> Option<Dag> {
        let rule = &self.rules[ri];
        let t = Instant::now();
        let cons = self.g.constraint_fn();
        self.u.begin();
        let r = self.u.add(rule.dag.clone());
        for (i, d) in dtrs.iter().enumerate() {
            let h = self.u.add(d.clone());
            if !self.u.unify(r + rule.dtrs[i], h, &self.g.ts, &cons) {
                self.stats.unify_failed += 1;
                self.stats.unify_time += t.elapsed();
                return None;
            }
        }
        self.stats.unify_time += t.elapsed();
        let t = Instant::now();
        let out = self.u.copy(r, &self.config.deleted_daughters);
        self.stats.copy_time += t.elapsed();
        if out.is_none() {
            self.stats.cyclic += 1;
        }
        out
    }
}

/// A derivation tree in the usual DELPH-IN bracketed notation.
pub fn derivation(
    g: &Grammar,
    rules: &[Rule],
    d: &Deriv,
    forms: &dyn Fn(&[usize]) -> String,
) -> String {
    match &d.kind {
        EdgeKind::Lex { inst, tokens } => format!(
            "({} {} {} (\"{}\"))",
            g.instances[*inst].name,
            d.start,
            d.end,
            forms(tokens)
        ),
        EdgeKind::Rule(ri) => {
            let kids: Vec<String> = d
                .daughters
                .iter()
                .map(|k| derivation(g, rules, k, forms))
                .collect();
            format!(
                "({} {} {} {})",
                rules[*ri].name,
                d.start,
                d.end,
                kids.join(" ")
            )
        }
    }
}
