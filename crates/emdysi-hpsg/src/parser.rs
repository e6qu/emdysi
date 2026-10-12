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
    /// The paths as a trie, so that a vector is read in one walk: each node
    /// has its arcs and the slot of the path ending there, if any.
    trie: Vec<TrieNode>,
}

/// A node of [`QuickCheck`]'s path trie: arcs to child nodes, and the slot
/// of the path ending here.
type TrieNode = (Vec<(FeatId, u32)>, Option<u32>);

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
        let mut qc = QuickCheck::default();
        for (_, path) in slots {
            qc.add_path(path);
        }
        qc
    }

    /// Add a path after the others, unless it is there already.
    pub fn add_path(&mut self, path: Vec<FeatId>) {
        if self.paths.contains(&path) {
            return;
        }
        if self.trie.is_empty() {
            self.trie.push((Vec::new(), None));
        }
        let mut at = 0usize;
        for &f in &path {
            at = match self.trie[at].0.iter().find(|a| a.0 == f) {
                Some(&(_, next)) => next as usize,
                None => {
                    let next = self.trie.len();
                    self.trie.push((Vec::new(), None));
                    self.trie[at].0.push((f, next as u32));
                    next
                }
            };
        }
        self.trie[at].1 = Some(self.paths.len() as u32);
        self.paths.push(path);
    }

    pub fn vector(&self, dag: &Dag, node: u32) -> Vec<TypeId> {
        let mut out = vec![TOP; self.paths.len()];
        if self.trie.is_empty() {
            return out;
        }
        let mut stack = vec![(0u32, node)];
        while let Some((t, n)) = stack.pop() {
            let (arcs, slot) = &self.trie[t as usize];
            if let Some(slot) = slot {
                out[*slot as usize] = dag.ty(n);
            }
            for &(f, next) in arcs {
                if let Some(m) = dag.arc(n, f) {
                    stack.push((next, m));
                }
            }
        }
        out
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
    /// A restriction a grammar's types cannot state: an edge of this rule
    /// whose first daughter is one of these lexical entries (instance
    /// indices) is the first daughter of a rule in `sealed_parents` (rule
    /// indices), or of no rule.
    pub sealed_heads: Vec<usize>,
    pub sealed_parents: Vec<usize>,
    /// Another such restriction, for a phrase of a head and an adjunct that
    /// still lacks its object: it is the first daughter only of these
    /// rules (rule indices), and only with a second daughter (the object)
    /// longer than its own second daughter (the adjunct) and at least
    /// five positions long (about four words). Empty: no restriction.
    pub heavy_object_parents: Vec<usize>,
    /// And another: an edge of this rule is only the second daughter of
    /// these rules (rule indices). Empty: no restriction.
    pub complement_of: Vec<usize>,
    /// And another: an edge of this rule needs a head daughter built by an
    /// adjunct-head rule (name starting `aj-hd`) whose adjunct ends in this
    /// lexical entry (instance index), a comma.
    pub paired_subject: Option<usize>,
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
            sealed_heads: Vec::new(),
            sealed_parents: Vec::new(),
            heavy_object_parents: Vec::new(),
            complement_of: Vec::new(),
            paired_subject: None,
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

/// Results of rule applications, shared by the searches of one input (a
/// narrow beam, then wider ones): a rule applied to daughters with the same
/// derivations gives the same structure, or fails again, so a search after
/// the first builds what the one before built without unifying. Keys are
/// (rule, daughter position or 2 for both, derivation hashes); failures
/// are kept as `None`.
#[derive(Default)]
pub struct UnifyMemo {
    results: HashMap<(usize, usize, u64, u64), Option<Arc<Dag>>>,
    /// Nodes in the structures kept, against the parser's node budget.
    nodes: usize,
}

/// A hash of a derivation step: a lexical entry over tokens (`rule` none)
/// or a rule over daughters' hashes.
fn sig_of(rule: Option<usize>, parts: &[u64]) -> u64 {
    // FNV-1a over the words, then a final mix.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut feed = |x: u64| {
        for b in x.to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    };
    feed(rule.map_or(u64::MAX, |r| r as u64));
    for &p in parts {
        feed(p);
    }
    h ^ (h >> 29)
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
    /// A hash of the edge's derivation (rules and lexical entries), which
    /// determines its structure: the key of [`UnifyMemo`].
    sig: u64,
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

/// The instantiations of edges made so far while unpacking: all at once
/// without a scorer ([`Parser::unpack`]), on demand with one
/// ([`Parser::lazy_inst`]).
#[derive(Default)]
struct Memo {
    eager: HashMap<usize, Vec<Inst>>,
    lazy: HashMap<usize, LazyEdge>,
}

/// One edge's instantiations, made on demand in the order
/// [`Parser::unpack`] would give them all at once: best first, at most
/// `unpack_beam`, merging the edge with the edges packed into it.
struct LazyEdge {
    out: Vec<Inst>,
    finished: bool,
    alts: Vec<AltGen>,
    /// The next instantiation of each alternative, if made.
    heads: Vec<Option<Inst>>,
    exhausted: Vec<bool>,
}

/// The instantiations of one edge (without those packed into it).
enum AltGen {
    Lex(Option<Inst>),
    Rule(RuleGen),
    None,
}

/// The successful combinations of a rule's daughter instantiations, best
/// first: the combinations are enumerated lazily over the grid of daughter
/// instantiation indices, ordered by score and then by index. A
/// combination scores the sum of its daughters' scores and the rule's
/// local score, which depends on the rules that built the daughters, so
/// scores are not monotone over the grid; a combination's sum plus the
/// best local score the rule can get (`max_rule`) bounds it and the
/// combinations after it, and a combination is tried only when no bound
/// left can beat it.
struct RuleGen {
    ri: usize,
    start: usize,
    end: usize,
    kids: Vec<usize>,
    max_rule: f64,
    /// Combinations by their bound.
    frontier: std::collections::BinaryHeap<Cand>,
    /// Combinations by their score.
    ready: std::collections::BinaryHeap<Cand>,
    seen: HashSet<Vec<usize>>,
    found: usize,
    /// First-daughter instantiations the rule does not unify with.
    failed_first: HashSet<usize>,
    started: bool,
}

/// A combination of daughter instantiation indices: higher scores first,
/// then the lexically smaller combination (the order of a stable sort of
/// the combinations generated in lexical order).
#[derive(Clone)]
struct Cand {
    score: f64,
    combo: Vec<usize>,
}

impl PartialEq for Cand {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == std::cmp::Ordering::Equal
    }
}
impl Eq for Cand {}
impl PartialOrd for Cand {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Cand {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.score
            .total_cmp(&o.score)
            .then_with(|| o.combo.cmp(&self.combo))
    }
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
    /// Lexical entries (instance indices) that lexical filtering leaves in
    /// place.
    pub unfiltered: Vec<usize>,
    /// Lattice spans (start and end vertex) whose lexical edges get this
    /// much added to their score.
    pub lexical_bonus: Vec<((u32, u32), f64)>,
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
    memo: Option<&'g std::cell::RefCell<UnifyMemo>>,
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
            memo: None,
        }
    }

    /// Reuse and record rule applications in `memo` (see [`UnifyMemo`]).
    pub fn with_memo(mut self, memo: &'g std::cell::RefCell<UnifyMemo>) -> Self {
        self.memo = Some(memo);
        self
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
                let bonus: f64 = self
                    .config
                    .lexical_bonus
                    .iter()
                    .filter(|(span, _)| *span == (it.start, it.end))
                    .map(|(_, b)| b)
                    .sum();
                let score = self.scorer.map_or(0.0, |s| s.lexical(it.inst)) + bonus;
                let mut parts = vec![it.inst as u64];
                parts.extend(it.tokens.iter().map(|&t| t as u64));
                let sig = sig_of(None, &parts);
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
                    sig,
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
                        sig: sig_of(Some(ri), &[e.sig]),
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
                let exempt = matches!(&self.chart[i].kind, EdgeKind::Lex { inst, .. }
                    if self.config.unfiltered.contains(inst));
                if !flat.entries[k].alive && !exempt {
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
                            // combined structure; not at all when a search
                            // before this one tried every pair (see
                            // UnifyMemo), whose results are taken from there
                            // in the same order.
                            let mut partial: Option<(Vec<TypeId>, Checkpoint)> = None;
                            let mut failed = false;
                            for c in cands {
                                if self.chart[id].state != EdgeState::Active {
                                    break;
                                }
                                let pair = if pos == 0 { [id, c] } else { [c, id] };
                                if self.memo_hit(ri, &pair, &mut agenda) {
                                    continue;
                                }
                                if failed {
                                    continue;
                                }
                                if partial.is_none() {
                                    let Some(q) = self.partial_qc(ri, pos, id) else {
                                        if traced(&self.rules[ri].name) {
                                            eprintln!("trace: edge {id} fails as daughter {pos}");
                                        }
                                        failed = true;
                                        continue;
                                    };
                                    // The unifier now holds the rule with
                                    // this daughter: each candidate is tried
                                    // from there.
                                    partial = Some((q, self.u.checkpoint()));
                                }
                                let Some((qc_other, cp)) = partial.as_mut() else {
                                    continue;
                                };
                                let slot = self.rules[ri].dtrs[other];
                                if !other_compatible(
                                    &mut self.u,
                                    &self.g.ts,
                                    self.qc,
                                    slot,
                                    qc_other,
                                    &self.chart[c].qc,
                                ) {
                                    self.stats.qc_filtered += 1;
                                    if traced(&self.rules[ri].name) {
                                        // The first path whose types clash.
                                        let at = (0..self.qc.paths.len()).find(|&i| {
                                            let y = self.chart[c].qc.get(i).copied().unwrap_or(TOP);
                                            let x = qc_other[i];
                                            y != TOP
                                                && x != TOP
                                                && x != UNSET
                                                && !self.u.compatible(&self.g.ts, &[x], &[y])
                                        });
                                        let why = at.map(|i| {
                                            let path: Vec<&str> = self.qc.paths[i]
                                                .iter()
                                                .map(|&f| self.g.feats.name(f))
                                                .collect();
                                            format!(
                                                ": {} {} vs {}",
                                                path.join(" "),
                                                self.g.ts.hier.name(qc_other[i]),
                                                self.g.ts.hier.name(self.chart[c].qc[i])
                                            )
                                        });
                                        eprintln!(
                                            "trace: edge {c} quick-check fails as daughter {other} with edge {id}{}",
                                            why.unwrap_or_default()
                                        );
                                    }
                                    continue;
                                }
                                let cp = &*cp;
                                self.try_other(ri, &pair, other, cp, &mut agenda);
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
                let pos = roots.iter().position(|r| self.unifies(r, &dag));
                if let EdgeKind::Rule(ri) = self.chart[id].kind {
                    if traced(&self.rules[ri].name) {
                        eprintln!(
                            "trace: spanning edge {id}: root {:?}, {} instantiations",
                            pos.map(|k| self.config.roots[k].0.clone()),
                            self.unpack(id, &mut HashMap::new()).len()
                        );
                        for (name, r) in &self.config.roots {
                            eprintln!("trace:   {name}: {:?}", self.g.clash_paths(r, &dag));
                        }
                    }
                }
                pos.map(|k| (if k < preferred { 0 } else { k }, id))
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
        let mut memo = Memo::default();
        // Two passes: readings under the preferred (strictest) root conditions,
        // then the others, so that the cap on readings does not crowd out
        // strict analyses with fragments.
        let mut taken: HashSet<(usize, usize)> = HashSet::new();
        'outer: for pass in 0..2 {
            for &id in &spanning {
                for k in 0.. {
                    if readings.len() >= self.config.max_readings || Instant::now() > self.deadline
                    {
                        break 'outer;
                    }
                    if taken.contains(&(id, k)) {
                        continue;
                    }
                    let Some(inst) = self.inst(id, k, &mut memo) else {
                        break;
                    };
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
    fn fragment_cover(&mut self, memo: &mut Memo) -> Option<Reading> {
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
            let inst = self.inst(id, 0, memo)?;
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
        if !self.chart[id].lexical && !Arc::ptr_eq(&self.chart[id].dag, &self.released) {
            // The node budget counts the structures the chart keeps.
            self.nodes -= self.chart[id].dag.nodes.len();
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
                self.nodes += dag.nodes.len();
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
            // Combinations are tried best first until `beam` succeed: as many
            // at a time as are still needed, so that the ones sharing a
            // first daughter can share its unification with the rule.
            let mut found = 0;
            let mut next = 0;
            while found < beam && next < combos.len() && Instant::now() <= self.deadline {
                let take = (beam - found).min(combos.len() - next);
                let chunk = &combos[next..next + take];
                next += take;
                let results = self.apply_combos(ri, &kid_insts, chunk);
                for ((score, c), m) in chunk.iter().zip(results) {
                    let Some(m) = m else { continue };
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
                        score: *score,
                    });
                }
            }
        }
        out.sort_by(|a, b| b.score.total_cmp(&a.score));
        out.truncate(beam);
        memo.insert(id, out.clone());
        out
    }

    /// The mothers of rule `ri` over combinations of daughter
    /// instantiations (indices into `kids`), in the order given. For a
    /// binary rule, the rule is unified with each distinct first daughter
    /// once, and each second daughter is tried from there.
    fn apply_combos(
        &mut self,
        ri: usize,
        kids: &[Vec<Inst>],
        combos: &[(f64, Vec<usize>)],
    ) -> Vec<Option<Arc<Dag>>> {
        let mut out: Vec<Option<Arc<Dag>>> = vec![None; combos.len()];
        self.stats.unpack_attempts += combos.len();
        if self.rules[ri].dtrs.len() != 2 || kids.len() != 2 {
            for (k, (_, c)) in combos.iter().enumerate() {
                let dags: Vec<Arc<Dag>> = c
                    .iter()
                    .enumerate()
                    .map(|(d, &j)| kids[d][j].dag.clone())
                    .collect();
                let dag_refs: Vec<&Arc<Dag>> = dags.iter().collect();
                let qcs: Vec<Vec<TypeId>> = dags.iter().map(|d| self.qc.vector(d, 0)).collect();
                let qc_refs: Vec<&Vec<TypeId>> = qcs.iter().collect();
                out[k] = self.apply(ri, &dag_refs, &qc_refs).map(Arc::new);
            }
        } else {
            let mut firsts: Vec<usize> = combos.iter().map(|(_, c)| c[0]).collect();
            firsts.sort_unstable();
            firsts.dedup();
            for first in firsts {
                if Instant::now() > self.deadline {
                    break;
                }
                let group: Vec<usize> = (0..combos.len())
                    .filter(|&k| combos[k].1[0] == first)
                    .collect();
                let rule = &self.rules[ri];
                let d0 = kids[0][first].dag.clone();
                if !self
                    .u
                    .compatible(&self.g.ts, &rule.dtr_qc[0], &self.qc.vector(&d0, 0))
                {
                    self.stats.qc_filtered += group.len();
                    continue;
                }
                let t = Instant::now();
                let cons = self.g.constraint_fn();
                self.u.begin();
                let r = self.u.add(rule.dag.clone());
                let h = self.u.add(d0);
                let ok = self.u.unify(r + rule.dtrs[0], h, &self.g.ts, &cons);
                self.stats.unify_time += t.elapsed();
                if !ok {
                    self.stats.unify_failed += group.len();
                    continue;
                }
                let cp = self.u.checkpoint();
                for k in group {
                    let d1 = kids[1][combos[k].1[1]].dag.clone();
                    let rule = &self.rules[ri];
                    if !self
                        .u
                        .compatible(&self.g.ts, &rule.dtr_qc[1], &self.qc.vector(&d1, 0))
                    {
                        self.stats.qc_filtered += 1;
                        continue;
                    }
                    let t = Instant::now();
                    let h = self.u.add(d1);
                    let ok = self.u.unify(r + rule.dtrs[1], h, &self.g.ts, &cons);
                    self.stats.unify_time += t.elapsed();
                    if ok {
                        let t = Instant::now();
                        let m = self.u.copy(r, &self.config.deleted_daughters);
                        self.stats.copy_time += t.elapsed();
                        if m.is_none() {
                            self.stats.cyclic += 1;
                        }
                        out[k] = m.map(Arc::new);
                    } else {
                        self.stats.unify_failed += 1;
                    }
                    self.u.rollback(&cp);
                }
            }
        }
        self.stats.unpack_failed += out.iter().filter(|m| m.is_none()).count();
        out
    }

    /// The `k`th instantiation of edge `id`, best first, as
    /// [`Parser::unpack`] lists them; with a scorer, made on demand.
    fn inst(&mut self, id: usize, k: usize, memo: &mut Memo) -> Option<Inst> {
        if self.scorer.is_none() {
            return self.unpack(id, &mut memo.eager).get(k).cloned();
        }
        let mut e = match memo.lazy.remove(&id) {
            Some(e) => e,
            None => self.lazy_edge(id),
        };
        while e.out.len() <= k && !e.finished && Instant::now() <= self.deadline {
            self.lazy_step(&mut e, memo);
        }
        let r = e.out.get(k).cloned();
        memo.lazy.insert(id, e);
        r
    }

    /// An edge and the edges packed into it, transitively, in the order
    /// [`Parser::unpack`] takes them.
    fn alternatives(&self, id: usize) -> Vec<usize> {
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
        alts
    }

    fn lazy_edge(&mut self, id: usize) -> LazyEdge {
        let alts: Vec<AltGen> = self
            .alternatives(id)
            .into_iter()
            .map(|a| self.alt_gen(a))
            .collect();
        let n = alts.len();
        LazyEdge {
            out: Vec::new(),
            finished: false,
            alts,
            heads: vec![None; n],
            exhausted: vec![false; n],
        }
    }

    fn alt_gen(&self, a: usize) -> AltGen {
        let e = &self.chart[a];
        if e.lexical {
            return AltGen::Lex(Some(Inst {
                dag: e.dag.clone(),
                deriv: Arc::new(self.chart_deriv(a)),
                score: e.score,
            }));
        }
        let EdgeKind::Rule(ri) = e.kind else {
            return AltGen::None;
        };
        let kids = e.daughters.clone();
        // The best local score of the rule, over the rules that can build
        // each daughter's instantiations.
        let max_rule = match self.scorer {
            Some(sc) => {
                let sets: Vec<Vec<Dtr>> = kids
                    .iter()
                    .map(|&d| {
                        self.alternatives(d)
                            .into_iter()
                            .map(|x| match &self.chart[x].kind {
                                EdgeKind::Lex { inst, .. } => Dtr::Lex(*inst),
                                EdgeKind::Rule(r) => Dtr::Rule(*r),
                                EdgeKind::Cover => Dtr::Rule(usize::MAX),
                            })
                            .collect()
                    })
                    .collect();
                let mut best = f64::NEG_INFINITY;
                let mut idx = vec![0; sets.len()];
                'odometer: loop {
                    let ds: Vec<Dtr> = idx.iter().enumerate().map(|(k, &j)| sets[k][j]).collect();
                    best = best.max(sc.rule(ri, &ds));
                    for k in (0..idx.len()).rev() {
                        idx[k] += 1;
                        if idx[k] < sets[k].len() {
                            continue 'odometer;
                        }
                        idx[k] = 0;
                    }
                    break;
                }
                best
            }
            None => 0.0,
        };
        AltGen::Rule(RuleGen {
            ri,
            start: e.start,
            end: e.end,
            kids,
            max_rule,
            frontier: std::collections::BinaryHeap::new(),
            ready: std::collections::BinaryHeap::new(),
            seen: HashSet::new(),
            found: 0,
            failed_first: HashSet::new(),
            started: false,
        })
    }

    /// Make the next instantiation of an edge: the best of its
    /// alternatives' next ones, the earlier alternative when tied (as a
    /// stable sort of their lists, one after the other, would order them).
    fn lazy_step(&mut self, e: &mut LazyEdge, memo: &mut Memo) {
        for i in 0..e.alts.len() {
            if e.heads[i].is_none() && !e.exhausted[i] {
                let next = match &mut e.alts[i] {
                    AltGen::Lex(inst) => inst.take(),
                    AltGen::None => None,
                    AltGen::Rule(r) => self.rule_next(r, memo),
                };
                e.exhausted[i] = next.is_none();
                e.heads[i] = next;
            }
        }
        let mut best: Option<usize> = None;
        for i in 0..e.heads.len() {
            if let Some(h) = &e.heads[i] {
                let better = match best {
                    None => true,
                    Some(b) => h
                        .score
                        .total_cmp(&e.heads[b].as_ref().map_or(f64::NEG_INFINITY, |x| x.score))
                        .is_gt(),
                };
                if better {
                    best = Some(i);
                }
            }
        }
        match best {
            Some(b) => {
                if let Some(inst) = e.heads[b].take() {
                    e.out.push(inst);
                }
                if e.out.len() >= self.config.unpack_beam.max(1) {
                    e.finished = true;
                }
            }
            None => e.finished = true,
        }
    }

    /// The sum of the scores of a combination's daughter instantiations,
    /// added in daughter order as [`Parser::unpack`] adds them.
    fn combo_sum(&mut self, kids: &[usize], combo: &[usize], memo: &mut Memo) -> f64 {
        let mut s = 0.0;
        for (k, &j) in combo.iter().enumerate() {
            s += self.inst(kids[k], j, memo).map_or(0.0, |i| i.score);
        }
        s
    }

    /// The next successful combination of a rule's daughter instantiations.
    fn rule_next(&mut self, r: &mut RuleGen, memo: &mut Memo) -> Option<Inst> {
        let beam = self.config.unpack_beam.max(1);
        let n = r.kids.len();
        if !r.started {
            r.started = true;
            let corner = vec![0; n];
            if (0..n).all(|k| self.inst(r.kids[k], 0, memo).is_some()) {
                let opt = self.combo_sum(&r.kids, &corner, memo) + r.max_rule;
                r.seen.insert(corner.clone());
                r.frontier.push(Cand {
                    score: opt,
                    combo: corner,
                });
            }
        }
        loop {
            if r.found >= beam || Instant::now() > self.deadline {
                return None;
            }
            // Score combinations until the best scored one cannot be beaten
            // by any not yet scored.
            loop {
                let expand = match (r.frontier.peek(), r.ready.peek()) {
                    (None, _) => false,
                    (Some(_), None) => true,
                    (Some(f), Some(c)) => c <= f,
                };
                if !expand {
                    break;
                }
                let Some(c) = r.frontier.pop() else { break };
                let sum = self.combo_sum(&r.kids, &c.combo, memo);
                let score = match self.scorer {
                    Some(sc) => {
                        let mut ds = Vec::with_capacity(n);
                        for (k, &j) in c.combo.iter().enumerate() {
                            let kind = self.inst(r.kids[k], j, memo).map(|i| i.deriv.kind.clone());
                            ds.push(match kind {
                                Some(EdgeKind::Lex { inst, .. }) => Dtr::Lex(inst),
                                Some(EdgeKind::Rule(x)) => Dtr::Rule(x),
                                _ => Dtr::Rule(usize::MAX),
                            });
                        }
                        sum + sc.rule(r.ri, &ds)
                    }
                    None => sum,
                };
                for k in 0..n {
                    let mut c2 = c.combo.clone();
                    c2[k] += 1;
                    if !r.seen.contains(&c2) && self.inst(r.kids[k], c2[k], memo).is_some() {
                        let opt = self.combo_sum(&r.kids, &c2, memo) + r.max_rule;
                        r.seen.insert(c2.clone());
                        r.frontier.push(Cand {
                            score: opt,
                            combo: c2,
                        });
                    }
                }
                r.ready.push(Cand {
                    score,
                    combo: c.combo,
                });
            }
            let c = r.ready.pop()?;
            if n == 2 && r.failed_first.contains(&c.combo[0]) {
                self.stats.unpack_attempts += 1;
                self.stats.unpack_failed += 1;
                continue;
            }
            let kids: Vec<Inst> = c
                .combo
                .iter()
                .enumerate()
                .filter_map(|(k, &j)| self.inst(r.kids[k], j, memo))
                .collect();
            match self.try_combo(r.ri, &kids) {
                Ok(m) => {
                    r.found += 1;
                    return Some(Inst {
                        dag: m.clone(),
                        deriv: Arc::new(Deriv {
                            kind: EdgeKind::Rule(r.ri),
                            start: r.start,
                            end: r.end,
                            daughters: kids.iter().map(|k| k.deriv.clone()).collect(),
                            dag: m,
                        }),
                        score: c.score,
                    });
                }
                Err(first_failed) => {
                    if first_failed && n == 2 {
                        r.failed_first.insert(c.combo[0]);
                    }
                }
            }
        }
    }

    /// The mother of rule `ri` over daughter instantiations, as
    /// [`Parser::apply_combos`] makes it; on failure, whether the rule
    /// already failed with the first daughter (of a binary rule).
    fn try_combo(&mut self, ri: usize, kids: &[Inst]) -> Result<Arc<Dag>, bool> {
        self.stats.unpack_attempts += 1;
        let res = if self.rules[ri].dtrs.len() != 2 || kids.len() != 2 {
            let dags: Vec<Arc<Dag>> = kids.iter().map(|k| k.dag.clone()).collect();
            let refs: Vec<&Arc<Dag>> = dags.iter().collect();
            let qcs: Vec<Vec<TypeId>> = dags.iter().map(|d| self.qc.vector(d, 0)).collect();
            let qrefs: Vec<&Vec<TypeId>> = qcs.iter().collect();
            self.apply(ri, &refs, &qrefs).map(Arc::new).ok_or(false)
        } else {
            let rule = &self.rules[ri];
            let d0 = kids[0].dag.clone();
            if !self
                .u
                .compatible(&self.g.ts, &rule.dtr_qc[0], &self.qc.vector(&d0, 0))
            {
                self.stats.qc_filtered += 1;
                Err(true)
            } else {
                let t = Instant::now();
                let cons = self.g.constraint_fn();
                self.u.begin();
                let r = self.u.add(rule.dag.clone());
                let h = self.u.add(d0);
                let ok = self.u.unify(r + rule.dtrs[0], h, &self.g.ts, &cons);
                self.stats.unify_time += t.elapsed();
                if !ok {
                    self.stats.unify_failed += 1;
                    Err(true)
                } else {
                    let d1 = kids[1].dag.clone();
                    let rule = &self.rules[ri];
                    if !self
                        .u
                        .compatible(&self.g.ts, &rule.dtr_qc[1], &self.qc.vector(&d1, 0))
                    {
                        self.stats.qc_filtered += 1;
                        Err(false)
                    } else {
                        let t = Instant::now();
                        let h = self.u.add(d1);
                        let ok = self.u.unify(r + rule.dtrs[1], h, &self.g.ts, &cons);
                        self.stats.unify_time += t.elapsed();
                        if ok {
                            let t = Instant::now();
                            let m = self.u.copy(r, &self.config.deleted_daughters);
                            self.stats.copy_time += t.elapsed();
                            if m.is_none() {
                                self.stats.cyclic += 1;
                            }
                            m.map(Arc::new).ok_or(false)
                        } else {
                            self.stats.unify_failed += 1;
                            Err(false)
                        }
                    }
                }
            }
        };
        if res.is_err() {
            self.stats.unpack_failed += 1;
        }
        res
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
        // EMDYSI_TRACE_FITS=<rule or lexical entry>: why edges built by that
        // rule, or of that entry, do not fit the daughters of the traced
        // rule (EMDYSI_TRACE_RULE).
        let built_by = match &self.chart[id].kind {
            EdgeKind::Rule(r) => Some(self.rules[*r].name.as_str()),
            EdgeKind::Lex { inst, .. } => Some(self.g.instances[*inst].name.as_str()),
            _ => None,
        };
        let explain =
            built_by.is_some_and(|b| std::env::var("EMDYSI_TRACE_FITS").is_ok_and(|w| w == b));
        for (dqc, slots) in &self.qc_groups {
            if self.u.compatible(&self.g.ts, dqc, qc) {
                for &(ri, pos) in slots {
                    fits[pos][ri / 64] |= 1 << (ri % 64);
                }
            } else if explain {
                for &(ri, pos) in slots {
                    if !traced(&self.rules[ri].name) {
                        continue;
                    }
                    let at = (0..dqc.len().min(qc.len())).find(|&i| {
                        dqc[i] != TOP
                            && qc[i] != TOP
                            && !self.u.compatible(&self.g.ts, &dqc[i..=i], &qc[i..=i])
                    });
                    if let Some(i) = at {
                        let path: Vec<&str> = self.qc.paths[i]
                            .iter()
                            .map(|&f| self.g.feats.name(f))
                            .collect();
                        eprintln!(
                            "trace: edge {id} does not fit daughter {pos}: {} {} vs {}",
                            path.join(" "),
                            self.g.ts.hier.name(dqc[i]),
                            self.g.ts.hier.name(qc[i])
                        );
                    }
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
            self.add_edge(ri, dtrs, Arc::new(dag), agenda);
        }
    }

    /// Apply a binary rule to two daughters from [`UnifyMemo`], if a search
    /// before this one tried them: whether it did.
    fn memo_hit(&mut self, ri: usize, dtrs: &[usize; 2], agenda: &mut Agenda) -> bool {
        let Some(m) = self.memo else { return false };
        let rule = &self.rules[ri];
        let (start, end) = (self.chart[dtrs[0]].start, self.chart[dtrs[1]].end);
        if rule.spanning_only && !(start == 0 && end == self.n) {
            return true;
        }
        let key = (ri, 2, self.chart[dtrs[0]].sig, self.chart[dtrs[1]].sig);
        let Some(found) = m.borrow().results.get(&key).cloned() else {
            return false;
        };
        self.stats.attempts += 1;
        match found {
            Some(dag) => self.add_edge(ri, dtrs, dag, agenda),
            None => self.stats.unify_failed += 1,
        }
        true
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
        let key = (ri, 2, self.chart[dtrs[0]].sig, self.chart[dtrs[1]].sig);
        let t = Instant::now();
        let cons = self.g.constraint_fn();
        let h = self.u.add(self.chart[dtrs[other]].dag.clone());
        let explain = traced(&rule.name);
        self.u.explain = explain;
        // The rule was the first structure added.
        let ok = self.u.unify(rule.dtrs[other], h, &self.g.ts, &cons);
        self.u.explain = false;
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
        if traced(&rule.name) {
            eprintln!(
                "trace: {:?}: {}",
                dtrs,
                if !ok {
                    let path: Vec<&str> = self
                        .u
                        .clash_path
                        .iter()
                        .rev()
                        .map(|&f| self.g.feats.name(f))
                        .collect();
                    let (x, y) = self.u.clash_types;
                    format!(
                        "unification fails: daughter {other} {} {} vs {}",
                        path.join(" "),
                        self.g.ts.hier.name(x),
                        self.g.ts.hier.name(y)
                    )
                } else if dag.is_none() {
                    "cyclic".into()
                } else {
                    "ok".into()
                }
            );
        }
        self.u.rollback(cp);
        let dag = dag.map(Arc::new);
        if let Some(m) = self.memo {
            let mut m = m.borrow_mut();
            let size = dag.as_ref().map_or(0, |d| d.nodes.len());
            if m.nodes + size <= self.config.max_nodes {
                m.nodes += size;
                m.results.insert(key, dag.clone());
            }
        }
        if let Some(dag) = dag {
            self.add_edge(ri, dtrs, dag, agenda);
        }
    }

    /// Add the mother of a rule to the chart and the agenda.
    fn add_edge(&mut self, ri: usize, dtrs: &[usize], dag: Arc<Dag>, agenda: &mut Agenda) {
        let breaks_seal = dtrs.iter().enumerate().any(|(i, &d)| {
            let EdgeKind::Rule(dr) = &self.chart[d].kind else {
                return false;
            };
            let rule = &self.rules[*dr];
            let sealed = !rule.sealed_heads.is_empty()
                && self.chart[d].daughters.first().is_some_and(|&h| {
                    matches!(&self.chart[h].kind, EdgeKind::Lex { inst, .. }
                        if rule.sealed_heads.contains(inst))
                });
            if sealed && !(i == 0 && rule.sealed_parents.contains(&ri)) {
                return true;
            }
            if !rule.complement_of.is_empty() && !(i == 1 && rule.complement_of.contains(&ri)) {
                return true;
            }
            if !rule.heavy_object_parents.is_empty() {
                let span = |e: usize| self.chart[e].end - self.chart[e].start;
                let heavy = i == 0
                    && rule.heavy_object_parents.contains(&ri)
                    && dtrs.get(1).is_some_and(|&o| {
                        let adjunct = self.chart[d].daughters.get(1).map_or(0, |&a| span(a));
                        span(o) >= 5 && span(o) > adjunct
                    });
                return !heavy;
            }
            false
        });
        if breaks_seal {
            return;
        }
        if let Some(comma) = self.rules[ri].paired_subject {
            let paired = dtrs.get(1).is_some_and(|&h| {
                let EdgeKind::Rule(hr) = self.chart[h].kind else {
                    return false;
                };
                if !self.rules[hr].name.starts_with("aj-hd") {
                    return false;
                }
                // The rightmost word of the adjunct.
                let Some(mut x) = self.chart[h].daughters.first().copied() else {
                    return false;
                };
                while let Some(&last) = self.chart[x].daughters.last() {
                    x = last;
                }
                matches!(self.chart[x].kind, EdgeKind::Lex { inst, .. } if inst == comma)
            });
            if !paired {
                return;
            }
        }
        let start = self.chart[dtrs[0]].start;
        let end = self.chart[*dtrs.last().unwrap()].end;
        self.nodes += dag.nodes.len();
        let score = self.rule_score(ri, dtrs);
        let id = self.chart.len();
        self.chart.push(Edge {
            start,
            end,
            qc: self.qc.vector(&dag, 0),
            dag,
            kind: EdgeKind::Rule(ri),
            daughters: dtrs.to_vec(),
            pending: Vec::new(),
            lexical: false,
            fits: Default::default(),
            state: EdgeState::Active,
            packed: Vec::new(),
            parents: Vec::new(),
            score,
            sig: sig_of(
                Some(ri),
                &dtrs.iter().map(|&d| self.chart[d].sig).collect::<Vec<_>>(),
            ),
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

/// Whether to trace attempts of rule `name` (`EMDYSI_TRACE_RULE=name`, for
/// grammar work): each attempt and where it fails is printed.
fn traced(name: &str) -> bool {
    static RULE: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    RULE.get_or_init(|| std::env::var("EMDYSI_TRACE_RULE").ok())
        .as_deref()
        == Some(name)
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
