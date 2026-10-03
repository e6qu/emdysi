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
use crate::typesys::{FeatId, TypeSystem};
use crate::unify::{Checkpoint, Unifier};

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
    Lex {
        inst: usize,
        tokens: Vec<usize>,
    },
    Rule(usize),
    /// A sequence of partial analyses covering the input, used when no
    /// complete analysis was found.
    Cover,
}

/// A daughter of a rule application, for scoring.
#[derive(Debug, Clone, Copy)]
pub enum Dtr {
    Lex(usize),
    Rule(usize),
}

/// Local scores for chart pruning and best-first unpacking: higher is
/// better. A derivation scores the sum of its local scores.
pub trait Scorer: Sync {
    /// Score of a lexical entry (an instance index).
    fn lexical(&self, inst: usize) -> f64;
    /// Score of applying rule `rule` to daughters `dtrs`.
    fn rule(&self, rule: usize, dtrs: &[Dtr]) -> f64;
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub start: usize,
    pub end: usize,
    pub dag: Arc<Dag>,
    pub kind: EdgeKind,
    pub daughters: Vec<usize>,
    /// Orthographic rule chains still to apply, innermost first, one per
    /// morphological analysis (lexical edges only). An empty chain means
    /// the edge is a complete word.
    pub pending: Vec<Vec<String>>,
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
    /// Model score of the best derivation found for this edge.
    pub score: f64,
}

impl Edge {
    /// Whether the edge is a complete word or phrase (no orthographic rule
    /// is required to apply).
    pub fn complete(&self) -> bool {
        !self.lexical || self.pending.iter().any(Vec::is_empty)
    }
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
    score: f64,
}

/// A quick-check slot not computed yet.
const UNSET: TypeId = TypeId::MAX;

/// Whether a candidate daughter's quick-check vector is compatible with
/// the slot `slot` of the structure in the unifier, filling in the slot's
/// vector `have` as needed.
fn other_compatible(
    u: &mut Unifier,
    ts: &TypeSystem,
    qc: &QuickCheck,
    slot: u32,
    have: &mut [TypeId],
    cand: &[TypeId],
) -> bool {
    for (i, &y) in cand.iter().enumerate() {
        if y == TOP {
            continue;
        }
        if have[i] == UNSET {
            have[i] = u.follow(slot, &qc.paths[i]).map_or(TOP, |n| u.node_type(n));
        }
        if have[i] != TOP && !u.compatible(ts, &have[i..=i], &[y]) {
            return false;
        }
    }
    true
}

fn bit(set: &[u64], i: usize) -> bool {
    set.get(i / 64).is_some_and(|w| w & (1 << (i % 64)) != 0)
}

#[derive(Clone)]
pub struct ParserConfig {
    pub deleted_daughters: Vec<FeatId>,
    pub roots: Vec<(String, Arc<Dag>)>,
    pub max_edges: usize,
    /// Upper bound on the total size (in nodes) of the feature structures
    /// in the chart, as a memory limit.
    pub max_nodes: usize,
    pub timeout: Duration,
    /// Features ignored when comparing edges for ambiguity packing; `None`
    /// disables packing.
    pub packing_restrictor: Option<Vec<FeatId>>,
    /// Upper bound on readings recovered by unpacking.
    pub max_readings: usize,
    /// When packing, compare edges as if their top type were this one
    /// (ACE's `generalize-edge-top-types`, e.g. `sign` for the ERG).
    pub packing_top_type: Option<TypeId>,
    /// Chart pruning: at most this many edges are processed per chart cell
    /// (span), the best-scoring first. Needs a scorer.
    pub cell_beam: Option<usize>,
    /// Chart pruning applies only to inputs longer than this many
    /// positions; shorter ones are parsed exhaustively.
    pub cell_beam_from: usize,
    /// The first this many root conditions are preferred: readings under
    /// them are recovered before any other.
    pub preferred_roots: usize,
    /// At most this many instantiations are kept per edge when unpacking,
    /// the best-scoring first (with a scorer).
    pub unpack_beam: usize,
    /// When no complete analysis is found, return a cover of the input by
    /// the largest partial analyses.
    pub fragments: bool,
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
    pub pruned: usize,
    pub packed_proactive: usize,
    pub packed_retroactive: usize,
    pub frozen: usize,
    /// Unifications tried while unpacking, and how many failed.
    pub unpack_attempts: usize,
    pub unpack_failed: usize,
}

pub struct ParseResult {
    pub stats: Stats,
    pub chart: Vec<Edge>,
    pub readings: Vec<Reading>,
    pub positions: usize,
    pub exhausted: bool,
    pub filtered_lexical: usize,
}

/// Syntactic agenda: shorter spans first, then left to right, so that an
/// edge is usually compared for packing with the edges it may subsume
/// before anything is built on top of it.
#[derive(Default)]
struct Agenda(std::collections::BinaryHeap<std::cmp::Reverse<(usize, usize, usize)>>);

impl Agenda {
    fn push(&mut self, id: usize, start: usize, end: usize) {
        self.0.push(std::cmp::Reverse((end - start, start, id)));
    }

    /// All queued edges of the next chart cell (same span).
    fn pop_cell(&mut self) -> Option<Vec<usize>> {
        let std::cmp::Reverse((len, start, id)) = self.0.pop()?;
        let mut out = vec![id];
        while let Some(&std::cmp::Reverse((l, s, i))) = self.0.peek() {
            if (l, s) != (len, start) {
                break;
            }
            self.0.pop();
            out.push(i);
        }
        Some(out)
    }
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
    /// Features ignored when packing, by feature id.
    restrict_mask: Vec<bool>,
    subsumer: Subsumer,
    u: Unifier,
    chart: Vec<Edge>,
    by_start: Vec<Vec<usize>>,
    by_end: Vec<Vec<usize>>,
    n: usize,
    deadline: Instant,
    stats: Stats,
    nodes: usize,
    scorer: Option<&'g dyn Scorer>,
    /// Stands in for the structure of a released edge.
    released: Arc<Dag>,
}

impl<'g> Parser<'g> {
    /// Use a scorer for chart pruning and best-first unpacking.
    pub fn with_scorer(mut self, scorer: &'g dyn Scorer) -> Self {
        self.scorer = Some(scorer);
        self
    }

    fn dtr(&self, id: usize) -> Dtr {
        match self.chart[id].kind {
            EdgeKind::Lex { inst, .. } => Dtr::Lex(inst),
            EdgeKind::Rule(ri) => Dtr::Rule(ri),
            EdgeKind::Cover => Dtr::Rule(usize::MAX),
        }
    }

    /// Score of a new rule edge over the given daughters.
    fn rule_score(&self, ri: usize, dtrs: &[usize]) -> f64 {
        let Some(sc) = self.scorer else { return 0.0 };
        let ds: Vec<Dtr> = dtrs.iter().map(|&d| self.dtr(d)).collect();
        dtrs.iter().map(|&d| self.chart[d].score).sum::<f64>() + sc.rule(ri, &ds)
    }

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
        let mut restrict_mask = vec![false; g.feats.len()];
        for &f in &restrictor {
            if let Some(m) = restrict_mask.get_mut(f as usize) {
                *m = true;
            }
        }
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
            restrict_mask,
            subsumer: Subsumer::default(),
            u: Unifier::new(),
            chart: Vec::new(),
            by_start: Vec::new(),
            by_end: Vec::new(),
            n: 0,
            deadline: Instant::now(),
            stats: Stats::default(),
            nodes: 0,
            scorer: None,
            released: Arc::new(Dag::atomic(TOP)),
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
        // Analyses of the same entry over the same tokens differ only in
        // their rule chains: one edge carries them all, so a shared prefix
        // of rules is applied once.
        let mut grouped: Vec<LexItem> = Vec::new();
        let mut index: HashMap<(usize, Vec<usize>), usize> = HashMap::new();
        let mut chains: Vec<Vec<Vec<String>>> = Vec::new();
        for it in items {
            match index.get(&(it.inst, it.tokens.clone())) {
                Some(&k) => {
                    if !chains[k].contains(&it.pending) {
                        chains[k].push(it.pending);
                    }
                }
                None => {
                    index.insert((it.inst, it.tokens.clone()), grouped.len());
                    chains.push(vec![it.pending.clone()]);
                    grouped.push(it);
                }
            }
        }
        let mut agenda: Vec<Edge> = grouped
            .into_iter()
            .zip(chains)
            .map(|(it, pending)| {
                let qc = self.qc.vector(&it.dag, 0);
                let score = self.scorer.map_or(0.0, |s| s.lexical(it.inst));
                Edge {
                    start: pos[&it.start],
                    end: pos[&it.end],
                    dag: it.dag,
                    kind: EdgeKind::Lex {
                        inst: it.inst,
                        tokens: it.tokens,
                    },
                    daughters: Vec::new(),
                    pending,
                    lexical: true,
                    qc,
                    fits: Default::default(),
                    state: EdgeState::Active,
                    packed: Vec::new(),
                    parents: Vec::new(),
                    score,
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
            let e_dtr = match e.kind {
                EdgeKind::Lex { inst, .. } => Dtr::Lex(inst),
                EdgeKind::Rule(r) => Dtr::Rule(r),
                EdgeKind::Cover => Dtr::Rule(usize::MAX),
            };
            let mut new = Vec::new();
            for (ri, rule) in self.rules.iter().enumerate() {
                if !rule.lexical || rule.dtrs.len() != 1 {
                    continue;
                }
                let pending = if rule.orth {
                    let rest: Vec<Vec<String>> = e
                        .pending
                        .iter()
                        .filter(|c| c.first() == Some(&rule.name))
                        .map(|c| c[1..].to_vec())
                        .collect();
                    if rest.is_empty() {
                        continue;
                    }
                    rest
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
                        score: e.score + self.scorer.map_or(0.0, |s| s.rule(ri, &[e_dtr])),
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
            .filter(|&i| self.chart[i].complete())
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
        let mut agenda = Agenda::default();
        for i in complete.into_iter().filter(|i| keep.contains(i)) {
            agenda.push(i, self.chart[i].start, self.chart[i].end);
        }
        let mut by_span: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
        let mut cell_count: HashMap<(usize, usize), usize> = HashMap::new();
        let cell_beam = self
            .config
            .cell_beam
            .filter(|_| self.scorer.is_some() && self.n > self.config.cell_beam_from);
        let mut batch: Vec<usize> = Vec::new();
        loop {
            if batch.is_empty() {
                let Some(mut cell) = agenda.pop_cell() else {
                    break;
                };
                // Best-scoring edges first; with a cell beam, the rest of
                // the cell is pruned.
                cell.sort_by(|&a, &b| {
                    self.chart[b]
                        .score
                        .total_cmp(&self.chart[a].score)
                        .then(a.cmp(&b))
                });
                cell.reverse();
                batch = cell;
            }
            let id = batch.pop().unwrap();
            if let Some(beam) = cell_beam {
                let key = (self.chart[id].start, self.chart[id].end);
                if self.chart[id].state == EdgeState::Active
                    && cell_count.get(&key).copied().unwrap_or(0) >= beam
                {
                    self.chart[id].state = EdgeState::Frozen;
                    self.release(id);
                    self.stats.pruned += 1;
                    continue;
                }
            }
            if self.chart.len() > self.config.max_edges
                || self.nodes > self.config.max_nodes
                || Instant::now() > self.deadline
            {
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
            *cell_count.entry((start, end)).or_default() += 1;
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
                            let Some(mut qc_other) = self.partial_qc(ri, pos, id) else {
                                continue;
                            };
                            // The unifier now holds the rule with this
                            // daughter: each candidate is tried from there.
                            let cp = self.u.checkpoint();
                            for c in cands {
                                if self.chart[id].state != EdgeState::Active {
                                    break;
                                }
                                let slot = self.rules[ri].dtrs[other];
                                if !other_compatible(
                                    &mut self.u,
                                    &self.g.ts,
                                    self.qc,
                                    slot,
                                    &mut qc_other,
                                    &self.chart[c].qc,
                                ) {
                                    self.stats.qc_filtered += 1;
                                    continue;
                                }
                                let pair = if pos == 0 { [id, c] } else { [c, id] };
                                self.try_other(ri, &pair, other, &cp, &mut agenda);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Unpacking (and the fragment fallback) gets a short grace period
        // when the search used up the time limit.
        let grace = (self.config.timeout / 5).min(Duration::from_secs(2));
        self.deadline = self.deadline.max(Instant::now() + grace);
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
        // Unpack edges that can satisfy the first (strictest) root
        // conditions first, the best-scoring first, so that a cap on
        // readings keeps the best analyses. Edges packed into a spanning
        // edge are more specific, so a root that fails on it fails on them.
        let mut spanning: Vec<(usize, usize)> = spanning
            .into_iter()
            .filter_map(|id| {
                let dag = self.chart[id].dag.clone();
                let roots: Vec<Arc<Dag>> =
                    self.config.roots.iter().map(|(_, r)| r.clone()).collect();
                let preferred = self.config.preferred_roots.max(1);
                roots
                    .iter()
                    .position(|r| self.unifies(r, &dag))
                    .map(|k| (if k < preferred { 0 } else { k }, id))
            })
            .collect();
        spanning.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then(self.chart[b.1].score.total_cmp(&self.chart[a.1].score))
                .then(a.1.cmp(&b.1))
        });
        let spanning: Vec<usize> = spanning.into_iter().map(|(_, id)| id).collect();
        let mut readings = Vec::new();
        let unpack_start = Instant::now();
        let mut memo: HashMap<usize, Vec<Inst>> = HashMap::new();
        // Two passes: readings under the preferred (strictest) root conditions,
        // then the others, so that the cap on readings does not crowd out
        // strict analyses with fragments.
        let mut taken: HashSet<(usize, usize)> = HashSet::new();
        'outer: for pass in 0..2 {
            for &id in &spanning {
                for (k, inst) in self.unpack(id, &mut memo).into_iter().enumerate() {
                    if readings.len() >= self.config.max_readings || Instant::now() > self.deadline
                    {
                        break 'outer;
                    }
                    if taken.contains(&(id, k)) {
                        continue;
                    }
                    let split = self
                        .config
                        .preferred_roots
                        .clamp(1, self.config.roots.len().max(1));
                    let split = split.min(self.config.roots.len());
                    let roots = if pass == 0 {
                        &self.config.roots[..split]
                    } else {
                        &self.config.roots[split..]
                    };
                    let roots = roots.to_vec();
                    for (name, root) in &roots {
                        if self.unifies(root, &inst.dag) {
                            taken.insert((id, k));
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
        }
        if readings.is_empty() && self.config.fragments {
            // The fallback has its own short time allowance.
            let grace = (self.config.timeout / 5).min(Duration::from_secs(2));
            self.deadline = Instant::now() + grace;
            if let Some(r) = self.fragment_cover(&mut memo) {
                readings.push(r);
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

    /// The fewest complete edges that together span the input (the
    /// best-scoring when tied), each with its best instantiation, as a
    /// `fragment` reading.
    fn fragment_cover(&mut self, memo: &mut HashMap<usize, Vec<Inst>>) -> Option<Reading> {
        let n = self.n;
        // best[j]: (pieces, score, last edge) of the best cover of 0..j.
        let mut best: Vec<Option<(usize, f64, usize)>> = vec![None; n + 1];
        best[0] = Some((0, 0.0, usize::MAX));
        let mut ending: Vec<Vec<usize>> = vec![Vec::new(); n + 1];
        for (id, e) in self.chart.iter().enumerate() {
            if e.state == EdgeState::Active && e.complete() && e.end > e.start {
                ending[e.end].push(id);
            }
        }
        for j in 1..=n {
            for &id in &ending[j] {
                let e = &self.chart[id];
                let Some((k, sc, _)) = best[e.start] else {
                    continue;
                };
                let cand = (k + 1, sc + e.score, id);
                let better = match best[j] {
                    None => true,
                    Some((bk, bs, _)) => cand.0 < bk || (cand.0 == bk && cand.1 > bs),
                };
                if better {
                    best[j] = Some(cand);
                }
            }
        }
        let mut pieces = Vec::new();
        let mut j = n;
        while j > 0 {
            let (_, _, id) = best[j]?;
            pieces.push(id);
            j = self.chart[id].start;
        }
        pieces.reverse();
        let mut kids = Vec::new();
        let mut dag: Option<(usize, Arc<Dag>)> = None;
        for id in pieces {
            let inst = self.unpack(id, memo).into_iter().next()?;
            let width = self.chart[id].end - self.chart[id].start;
            if dag.as_ref().is_none_or(|(w, _)| width > *w) {
                dag = Some((width, inst.dag.clone()));
            }
            kids.push(inst.deriv);
        }
        let dag = dag?.1;
        Some(Reading {
            root: "fragment".to_string(),
            deriv: Arc::new(Deriv {
                kind: EdgeKind::Cover,
                start: 0,
                end: n,
                daughters: kids,
                dag: dag.clone(),
            }),
            dag,
        })
    }

    /// Ambiguity packing (Oepen & Carroll 2000). Returns true if the edge
    /// was packed into an existing, more general edge.
    fn pack(
        &mut self,
        id: usize,
        by_span: &HashMap<(usize, usize), Vec<usize>>,
        agenda: &mut Agenda,
    ) -> bool {
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
            self.stats.subsumption_checks += 1;
            let (o_subsumes, id_subsumes) = self.subsumer.check_restricted(
                &self.g.ts,
                &self.chart[o].dag,
                &self.chart[id].dag,
                &self.restrict_mask,
                self.config.packing_top_type.is_some(),
            );
            if o_subsumes {
                self.chart[id].state = EdgeState::Packed(o);
                self.chart[o].packed.push(id);
                self.release(id);
                self.stats.packed_proactive += 1;
                return true;
            }
            if id_subsumes {
                self.chart[o].state = EdgeState::Packed(id);
                self.chart[id].packed.push(o);
                self.release(o);
                self.stats.packed_retroactive += 1;
                self.freeze_parents(o, agenda);
            }
        }
        false
    }

    /// Drop the structure of a syntactic edge that is packed or frozen: it
    /// is only needed again if the edge is reactivated, and unpacking
    /// rebuilds what it needs from the daughters.
    fn release(&mut self, id: usize) {
        if !self.chart[id].lexical {
            self.chart[id].dag = self.released.clone();
        }
    }

    /// Rebuild the structure of a released edge from its daughters, which
    /// are active (otherwise the edge would be frozen). Returns false if
    /// that fails, which should not happen.
    fn restore(&mut self, id: usize) -> bool {
        if !Arc::ptr_eq(&self.chart[id].dag, &self.released) {
            return true;
        }
        let EdgeKind::Rule(ri) = self.chart[id].kind else {
            return false;
        };
        let dags: Vec<Arc<Dag>> = self.chart[id]
            .daughters
            .iter()
            .map(|&d| self.chart[d].dag.clone())
            .collect();
        match self.unify_rule(ri, &dags) {
            Some(dag) => {
                self.chart[id].dag = Arc::new(dag);
                true
            }
            None => false,
        }
    }

    /// Invalidate everything built from an edge that was packed
    /// retroactively; edges packed into invalidated ones get a second chance.
    fn freeze_parents(&mut self, id: usize, agenda: &mut Agenda) {
        let mut stack: Vec<usize> = self.chart[id].parents.clone();
        while let Some(p) = stack.pop() {
            match self.chart[p].state {
                EdgeState::Frozen => continue,
                EdgeState::Packed(host) => self.chart[host].packed.retain(|&x| x != p),
                EdgeState::Active => {}
            }
            self.chart[p].state = EdgeState::Frozen;
            self.release(p);
            self.stats.frozen += 1;
            stack.extend(self.chart[p].parents.iter().copied());
            for q in std::mem::take(&mut self.chart[p].packed) {
                if self.restore(q) {
                    self.chart[q].state = EdgeState::Active;
                    agenda.push(q, self.chart[q].start, self.chart[q].end);
                } else {
                    self.chart[q].state = EdgeState::Frozen;
                }
            }
        }
    }

    /// Instantiations of an edge and the edges packed into it, best first
    /// when there is a scorer (at most `unpack_beam` of them).
    fn unpack(&mut self, id: usize, memo: &mut HashMap<usize, Vec<Inst>>) -> Vec<Inst> {
        let beam = if self.scorer.is_some() {
            self.config.unpack_beam.max(1)
        } else {
            1000
        };
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
        let mut out: Vec<Inst> = Vec::new();
        for a in alts {
            if Instant::now() > self.deadline {
                break;
            }
            let e = &self.chart[a];
            if e.lexical {
                out.push(Inst {
                    dag: e.dag.clone(),
                    deriv: Arc::new(self.chart_deriv(a)),
                    score: e.score,
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
            // Combinations of daughter instantiations, best first.
            let mut combos: Vec<(f64, Vec<usize>)> = vec![(0.0, Vec::new())];
            for k in &kid_insts {
                let mut next = Vec::new();
                for (sc, c) in &combos {
                    for (j, inst) in k.iter().enumerate() {
                        let mut c2 = c.clone();
                        c2.push(j);
                        next.push((sc + inst.score, c2));
                    }
                }
                combos = next;
            }
            if let Some(sc) = self.scorer {
                for (score, c) in combos.iter_mut() {
                    let ds: Vec<Dtr> = c
                        .iter()
                        .enumerate()
                        .map(|(k, &j)| match kid_insts[k][j].deriv.kind {
                            EdgeKind::Lex { inst, .. } => Dtr::Lex(inst),
                            EdgeKind::Rule(r) => Dtr::Rule(r),
                            EdgeKind::Cover => Dtr::Rule(usize::MAX),
                        })
                        .collect();
                    *score += sc.rule(ri, &ds);
                }
                combos.sort_by(|a, b| b.0.total_cmp(&a.0));
            }
            let mut found = 0;
            for (score, c) in combos {
                if found >= beam || Instant::now() > self.deadline {
                    break;
                }
                let dags: Vec<Arc<Dag>> = c
                    .iter()
                    .enumerate()
                    .map(|(k, &j)| kid_insts[k][j].dag.clone())
                    .collect();
                let dag_refs: Vec<&Arc<Dag>> = dags.iter().collect();
                let qcs: Vec<Vec<TypeId>> = dags.iter().map(|d| self.qc.vector(d, 0)).collect();
                let qc_refs: Vec<&Vec<TypeId>> = qcs.iter().collect();
                self.stats.unpack_attempts += 1;
                let m = self.apply(ri, &dag_refs, &qc_refs).map(Arc::new);
                self.stats.unpack_failed += m.is_none() as usize;
                if let Some(m) = m {
                    found += 1;
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
                        score,
                    });
                }
            }
        }
        out.sort_by(|a, b| b.score.total_cmp(&a.score));
        out.truncate(beam);
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
            if self.u.compatible(&self.g.ts, dqc, qc) {
                for &(ri, pos) in slots {
                    fits[pos][ri / 64] |= 1 << (ri % 64);
                }
            }
        }
        self.chart[id].fits = fits;
    }

    /// Unify rule `ri` with edge `id` as daughter `pos` and return an empty
    /// quick-check vector for the other daughter slot (see
    /// [`other_compatible`]), or `None` if the edge does not fit the rule
    /// at all.
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
        // Filled in as candidates are checked: most fail on the first few
        // paths.
        Some(vec![UNSET; self.qc.paths.len()])
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

    fn try_rule(&mut self, ri: usize, dtrs: &[usize], agenda: &mut Agenda) {
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
            self.add_edge(ri, dtrs, dag, agenda);
        }
    }

    /// Try a binary rule whose daughter other than `other` is already
    /// unified into the unifier (see [`Parser::partial_qc`]), then return
    /// to checkpoint `cp`.
    fn try_other(
        &mut self,
        ri: usize,
        dtrs: &[usize; 2],
        other: usize,
        cp: &Checkpoint,
        agenda: &mut Agenda,
    ) {
        let rule = &self.rules[ri];
        let (start, end) = (self.chart[dtrs[0]].start, self.chart[dtrs[1]].end);
        if rule.spanning_only && !(start == 0 && end == self.n) {
            return;
        }
        self.stats.attempts += 1;
        let t = Instant::now();
        let cons = self.g.constraint_fn();
        let h = self.u.add(self.chart[dtrs[other]].dag.clone());
        // The rule was the first structure added.
        let ok = self.u.unify(rule.dtrs[other], h, &self.g.ts, &cons);
        self.stats.unify_time += t.elapsed();
        let dag = if ok {
            let t = Instant::now();
            let out = self.u.copy(0, &self.config.deleted_daughters);
            self.stats.copy_time += t.elapsed();
            if out.is_none() {
                self.stats.cyclic += 1;
            }
            out
        } else {
            self.stats.unify_failed += 1;
            None
        };
        self.u.rollback(cp);
        if let Some(dag) = dag {
            self.add_edge(ri, dtrs, dag, agenda);
        }
    }

    /// Add the mother of a rule to the chart and the agenda.
    fn add_edge(&mut self, ri: usize, dtrs: &[usize], dag: Dag, agenda: &mut Agenda) {
        let start = self.chart[dtrs[0]].start;
        let end = self.chart[*dtrs.last().unwrap()].end;
        self.nodes += dag.nodes.len();
        let score = self.rule_score(ri, dtrs);
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
            score,
        });
        for &d in dtrs {
            self.chart[d].parents.push(id);
        }
        agenda.push(id, start, end);
    }

    /// Unify a rule with its daughters and copy out the mother, after a
    /// quick check of each daughter.
    fn apply(&mut self, ri: usize, dtrs: &[&Arc<Dag>], qcs: &[&Vec<TypeId>]) -> Option<Dag> {
        self.stats.attempts += 1;
        let rule = &self.rules[ri];
        for (i, qc) in qcs.iter().enumerate() {
            if !self.u.compatible(&self.g.ts, &rule.dtr_qc[i], qc) {
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
        EdgeKind::Cover => {
            let kids: Vec<String> = d
                .daughters
                .iter()
                .map(|k| derivation(g, rules, k, forms))
                .collect();
            format!("(fragments {} {} {})", d.start, d.end, kids.join(" "))
        }
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
