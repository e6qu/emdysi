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
use crate::dag::Dag;
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
}

fn bit(set: &[u64], i: usize) -> bool {
    set.get(i / 64).is_some_and(|w| w & (1 << (i % 64)) != 0)
}

pub struct ParserConfig {
    pub deleted_daughters: Vec<FeatId>,
    pub roots: Vec<(String, Arc<Dag>)>,
    pub max_edges: usize,
    pub timeout: Duration,
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
}

pub struct ParseResult {
    pub stats: Stats,
    pub chart: Vec<Edge>,
    /// Edges spanning the input that satisfy a root condition, with the root.
    pub readings: Vec<(usize, String)>,
    pub positions: usize,
    pub exhausted: bool,
    pub filtered_lexical: usize,
}

pub struct Parser<'g> {
    pub g: &'g Grammar,
    pub rules: &'g [Rule],
    pub qc: &'g QuickCheck,
    pub config: &'g ParserConfig,
    pub lexical_filter: &'g [MapRule],
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
        Parser {
            g,
            rules,
            qc,
            config,
            lexical_filter,
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

        // Phase 3: syntactic rules.
        let mut agenda: Vec<usize> = complete.into_iter().filter(|i| keep.contains(i)).collect();
        agenda.sort_unstable_by(|a, b| b.cmp(a));
        while let Some(id) = agenda.pop() {
            if self.chart.len() > self.config.max_edges || Instant::now() > self.deadline {
                exhausted = true;
                break;
            }
            let (start, end) = (self.chart[id].start, self.chart[id].end);
            self.compute_fits(id);
            self.by_start[start].push(id);
            self.by_end[end].push(id);
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
                        if bit(&self.chart[id].fits[0], ri) {
                            let right: Vec<usize> = self.by_start[end]
                                .iter()
                                .copied()
                                .filter(|&r| bit(&self.chart[r].fits[1], ri))
                                .collect();
                            for r in right {
                                self.try_rule(ri, &[id, r], &mut agenda);
                            }
                        }
                        if bit(&self.chart[id].fits[1], ri) {
                            let left: Vec<usize> = self.by_end[start]
                                .iter()
                                .copied()
                                .filter(|&l| l != id && bit(&self.chart[l].fits[0], ri))
                                .collect();
                            for l in left {
                                self.try_rule(ri, &[l, id], &mut agenda);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut readings = Vec::new();
        let spanning: Vec<usize> = self
            .by_start
            .first()
            .map(|v| {
                v.iter()
                    .copied()
                    .filter(|&i| self.chart[i].end == self.n)
                    .collect()
            })
            .unwrap_or_default();
        for id in spanning {
            for (name, root) in &self.config.roots {
                if self.unifies(root, &self.chart[id].dag.clone()) {
                    readings.push((id, name.clone()));
                    break;
                }
            }
        }
        ParseResult {
            stats: self.stats,
            chart: self.chart,
            readings,
            positions: self.n,
            exhausted,
            filtered_lexical: filtered,
        }
    }

    /// Record which syntactic rule daughters an edge passes the quick check for.
    fn compute_fits(&mut self, id: usize) {
        let words = self.rules.len().div_ceil(64);
        let mut fits = [vec![0u64; words], vec![0u64; words]];
        let qc = &self.chart[id].qc;
        for (ri, rule) in self.rules.iter().enumerate() {
            if rule.lexical {
                continue;
            }
            for (pos, dqc) in rule.dtr_qc.iter().enumerate().take(2) {
                if QuickCheck::compatible(self.g, dqc, qc) {
                    fits[pos][ri / 64] |= 1 << (ri % 64);
                }
            }
        }
        self.chart[id].fits = fits;
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
        let dags: Vec<Arc<Dag>> = dtrs.iter().map(|&d| self.chart[d].dag.clone()).collect();
        let qcs: Vec<Vec<TypeId>> = dtrs.iter().map(|&d| self.chart[d].qc.clone()).collect();
        let dag_refs: Vec<&Arc<Dag>> = dags.iter().collect();
        let qc_refs: Vec<&Vec<TypeId>> = qcs.iter().collect();
        if let Some(dag) = self.apply(ri, &dag_refs, &qc_refs) {
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
            });
            agenda.push(id);
        }
    }

    /// Unify a rule with its daughters and copy out the mother.
    fn apply(&mut self, ri: usize, dtrs: &[&Arc<Dag>], qcs: &[&Vec<TypeId>]) -> Option<Dag> {
        let rule = &self.rules[ri];
        self.stats.attempts += 1;
        for (i, qc) in qcs.iter().enumerate() {
            if !QuickCheck::compatible(self.g, &rule.dtr_qc[i], qc) {
                self.stats.qc_filtered += 1;
                return None;
            }
        }
        let t = Instant::now();
        let cons = self.g.constraint_fn();
        self.u.begin();
        let r = self.u.add(rule.dag.clone());
        for (i, d) in dtrs.iter().enumerate() {
            let h = self.u.add((*d).clone());
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
    chart: &[Edge],
    id: usize,
    forms: &dyn Fn(&[usize]) -> String,
) -> String {
    let e = &chart[id];
    match &e.kind {
        EdgeKind::Lex { inst, tokens } => format!(
            "({} {} {} (\"{}\"))",
            g.instances[*inst].name,
            e.start,
            e.end,
            forms(tokens)
        ),
        EdgeKind::Rule(ri) => {
            let kids: Vec<String> = e
                .daughters
                .iter()
                .map(|&d| derivation(g, rules, chart, d, forms))
                .collect();
            format!(
                "({} {} {} {})",
                rules[*ri].name,
                e.start,
                e.end,
                kids.join(" ")
            )
        }
    }
}
