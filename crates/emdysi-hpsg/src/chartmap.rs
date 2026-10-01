//! Chart mapping (Adolphs et al. 2008): rules that rewrite a lattice of
//! feature structures, used for token mapping before lexical lookup and for
//! lexical filtering afterwards.
//!
//! A rule names `+CONTEXT` items (which must be present and stay), `+INPUT`
//! items (which are consumed) and `+OUTPUT` items (which are added), and a
//! `+POSITION` string relating their spans:
//!
//! - `A<B`: A ends where B starts;
//! - `A<<B`: A ends no later than B starts;
//! - `A@B`: same span (an output related to several items covers them all);
//! - `^` and `$` stand for the start and end of the lattice.
//!
//! Items match lattice entries by unification; regex literals in a rule
//! match string values in full, and outputs may quote their capture groups
//! with `${I1:+FORM:1}`, optionally wrapped in `lc(...)` or `uc(...)`.
//!
//! Rules apply in order; each rule fires repeatedly until no new match
//! remains. A combination of entries fires a rule at most once. Outputs
//! identical to an entry already in the lattice (same structure and span)
//! are not added again, and inputs identical to one of the outputs are kept,
//! so a firing that would add and remove nothing is a no-op.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::dag::Dag;
use crate::grammar::Grammar;
use crate::typesys::{FeatId, LiteralKind, TypeSystem};
use crate::unify::Unifier;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Ref {
    Start,
    End,
    Context(usize),
    Input(usize),
    Output(usize),
}

#[derive(Debug, Clone, Copy)]
enum Pos {
    /// `<`
    Adjacent(Ref, Ref),
    /// `<<`
    Precedes(Ref, Ref),
    /// `@`
    Same(Ref, Ref),
}

#[derive(Debug, Clone)]
struct RegexSlot {
    /// Index into contexts followed by inputs.
    item: usize,
    path: Vec<FeatId>,
}

#[derive(Debug, Clone)]
pub struct MapRule {
    pub name: String,
    pub dag: Arc<Dag>,
    contexts: Vec<u32>,
    inputs: Vec<u32>,
    outputs: Vec<u32>,
    positions: Vec<Pos>,
    regex_slots: Vec<RegexSlot>,
}

#[derive(Debug)]
pub struct MapRuleError(pub String);

/// Feature paths that locate the parts of a chart-mapping rule.
pub struct MapPaths {
    pub context: FeatId,
    pub input: FeatId,
    pub output: FeatId,
    pub position: FeatId,
}

impl MapPaths {
    pub fn erg(g: &Grammar) -> Option<MapPaths> {
        Some(MapPaths {
            context: g.feat("+CONTEXT")?,
            input: g.feat("+INPUT")?,
            output: g.feat("+OUTPUT")?,
            position: g.feat("+POSITION")?,
        })
    }
}

fn list_items(g: &Grammar, dag: &Dag, mut n: u32) -> Vec<u32> {
    let mut items = Vec::new();
    while let Some(first) = dag.arc(n, g.lists.first) {
        items.push(first);
        match dag.arc(n, g.lists.rest) {
            Some(r) => n = r,
            None => break,
        }
    }
    items
}

impl MapRule {
    pub fn compile(
        g: &Grammar,
        name: &str,
        dag: Arc<Dag>,
        paths: &MapPaths,
    ) -> Result<MapRule, MapRuleError> {
        let items = |f: FeatId| {
            dag.arc(0, f)
                .map(|n| list_items(g, &dag, n))
                .unwrap_or_default()
        };
        let contexts = items(paths.context);
        let inputs = items(paths.input);
        let outputs = items(paths.output);
        let pos_str = dag
            .arc(0, paths.position)
            .and_then(|n| g.ts.literal_value(dag.ty(n)))
            .map(|(_, s)| s.to_string())
            .unwrap_or_default();
        let positions = parse_positions(&pos_str).map_err(MapRuleError)?;
        let mut regex_slots = Vec::new();
        for (k, &item) in contexts.iter().chain(inputs.iter()).enumerate() {
            // Breadth-first so that each regex is reached by its shortest path.
            let mut seen = HashSet::new();
            let mut queue = vec![(item, Vec::<FeatId>::new())];
            let mut i = 0;
            while i < queue.len() {
                let (n, path) = queue[i].clone();
                i += 1;
                if !seen.insert(n) {
                    continue;
                }
                if let Some((LiteralKind::Regex, _)) = g.ts.literal_value(dag.ty(n)) {
                    regex_slots.push(RegexSlot {
                        item: k,
                        path: path.clone(),
                    });
                }
                for &(f, v) in dag.arcs(n) {
                    let mut p = path.clone();
                    p.push(f);
                    queue.push((v, p));
                }
            }
        }
        Ok(MapRule {
            name: name.to_string(),
            dag,
            contexts,
            inputs,
            outputs,
            positions,
            regex_slots,
        })
    }
}

fn parse_ref(s: &str) -> Result<Ref, String> {
    let s = s.trim();
    match s {
        "^" => return Ok(Ref::Start),
        "$" => return Ok(Ref::End),
        _ => {}
    }
    let (kind, num) = s.split_at(1);
    let n: usize = num
        .parse()
        .map_err(|_| format!("bad position item {s:?}"))?;
    if n == 0 {
        return Err(format!("bad position item {s:?}"));
    }
    match kind {
        "C" => Ok(Ref::Context(n - 1)),
        "I" => Ok(Ref::Input(n - 1)),
        "O" => Ok(Ref::Output(n - 1)),
        _ => Err(format!("bad position item {s:?}")),
    }
}

fn parse_positions(s: &str) -> Result<Vec<Pos>, String> {
    let mut out = Vec::new();
    for clause in s.split(',').map(str::trim).filter(|c| !c.is_empty()) {
        // Tokenize a chain like `I1<I2<<I3@O1`.
        let mut refs = Vec::new();
        let mut ops = Vec::new();
        let mut cur = String::new();
        let chars: Vec<char> = clause.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            match chars[i] {
                '<' if chars.get(i + 1) == Some(&'<') => {
                    refs.push(parse_ref(&cur)?);
                    cur.clear();
                    ops.push("<<");
                    i += 2;
                    continue;
                }
                '<' => {
                    refs.push(parse_ref(&cur)?);
                    cur.clear();
                    ops.push("<");
                }
                '@' => {
                    refs.push(parse_ref(&cur)?);
                    cur.clear();
                    ops.push("@");
                }
                c => cur.push(c),
            }
            i += 1;
        }
        refs.push(parse_ref(&cur)?);
        for (k, op) in ops.iter().enumerate() {
            let (a, b) = (refs[k], refs[k + 1]);
            out.push(match *op {
                "<" => Pos::Adjacent(a, b),
                "<<" => Pos::Precedes(a, b),
                _ => Pos::Same(a, b),
            });
        }
    }
    Ok(out)
}

/// An entry in the lattice: a feature structure spanning two vertices.
#[derive(Debug, Clone)]
pub struct Entry {
    pub dag: Arc<Dag>,
    pub start: u32,
    pub end: u32,
    pub alive: bool,
}

/// A lattice of feature structures. Vertices are ordered by a numeric key so
/// that new vertices can be created between existing ones.
#[derive(Debug, Clone, Default)]
pub struct Lattice {
    pub entries: Vec<Entry>,
    keys: Vec<f64>,
}

impl Lattice {
    pub fn add_vertex(&mut self, key: f64) -> u32 {
        self.keys.push(key);
        (self.keys.len() - 1) as u32
    }

    pub fn key(&self, v: u32) -> f64 {
        self.keys[v as usize]
    }

    pub fn vertex_count(&self) -> usize {
        self.keys.len()
    }

    pub fn add(&mut self, dag: Arc<Dag>, start: u32, end: u32) -> usize {
        self.entries.push(Entry {
            dag,
            start,
            end,
            alive: true,
        });
        self.entries.len() - 1
    }

    pub fn alive(&self) -> impl Iterator<Item = (usize, &Entry)> {
        self.entries.iter().enumerate().filter(|(_, e)| e.alive)
    }

    fn first_vertex(&self) -> Option<u32> {
        self.alive()
            .map(|(_, e)| e.start)
            .min_by(|a, b| self.key(*a).total_cmp(&self.key(*b)))
    }

    fn last_vertex(&self) -> Option<u32> {
        self.alive()
            .map(|(_, e)| e.end)
            .max_by(|a, b| self.key(*a).total_cmp(&self.key(*b)))
    }

    /// Vertices in order of their keys.
    pub fn ordered_vertices(&self) -> Vec<u32> {
        let mut v: Vec<u32> = (0..self.keys.len() as u32).collect();
        v.sort_by(|a, b| self.key(*a).total_cmp(&self.key(*b)));
        v
    }
}

fn dag_eq(a: &Dag, b: &Dag) -> bool {
    a.nodes.len() == b.nodes.len()
        && a.arcs == b.arcs
        && a.nodes
            .iter()
            .zip(b.nodes.iter())
            .all(|(x, y)| x.ty == y.ty && x.arc_start == y.arc_start && x.arc_len == y.arc_len)
}

struct Match {
    ids: Vec<usize>,
    outputs: Vec<Dag>,
}

/// Apply chart-mapping rules to a lattice.
pub struct ChartMapper<'g> {
    pub g: &'g Grammar,
    pub rules: &'g [MapRule],
    pub unifier: Unifier,
    /// Rule names in the order they fired, for tracing.
    pub trace: Option<Vec<String>>,
}

const MAX_FIRINGS_PER_RULE: usize = 10_000;

impl<'g> ChartMapper<'g> {
    pub fn new(g: &'g Grammar, rules: &'g [MapRule]) -> Self {
        ChartMapper {
            g,
            rules,
            unifier: Unifier::new(),
            trace: None,
        }
    }

    pub fn apply(&mut self, lat: &mut Lattice) {
        let mut fired: HashSet<(usize, Vec<usize>)> = HashSet::new();
        for ri in 0..self.rules.len() {
            for _ in 0..MAX_FIRINGS_PER_RULE {
                let Some(m) = self.find_match(ri, lat, &fired) else {
                    break;
                };
                fired.insert((ri, m.ids.clone()));
                if self.fire(ri, lat, m) {
                    if let Some(t) = &mut self.trace {
                        t.push(self.rules[ri].name.clone());
                    }
                }
            }
        }
    }

    /// Whether every regex slot of the given items resolved to a string.
    fn regexes_resolved(
        &self,
        rule: &MapRule,
        root: u32,
        items: &[u32],
        which: impl Fn(usize) -> bool,
    ) -> bool {
        rule.regex_slots.iter().filter(|s| which(s.item)).all(|s| {
            let Some(h) = self.unifier.follow(root + items[s.item], &s.path) else {
                return false;
            };
            matches!(
                self.g.ts.literal_value(self.unifier.node_type(h)),
                Some((LiteralKind::Str, _))
            )
        })
    }

    fn item_matches(&mut self, rule: &MapRule, items: &[u32], k: usize, entry: &Entry) -> bool {
        let cons = self.g.constraint_fn();
        let u = &mut self.unifier;
        u.begin();
        let r = u.add(rule.dag.clone());
        let h = u.add(entry.dag.clone());
        if !u.unify(r + items[k], h, &self.g.ts, &cons) {
            return false;
        }
        self.regexes_resolved(rule, r, items, |i| i == k)
    }

    fn find_match(
        &mut self,
        ri: usize,
        lat: &Lattice,
        fired: &HashSet<(usize, Vec<usize>)>,
    ) -> Option<Match> {
        let rule = &self.rules[ri];
        let items: Vec<u32> = rule
            .contexts
            .iter()
            .chain(rule.inputs.iter())
            .copied()
            .collect();
        if items.is_empty() {
            return None;
        }
        let mut cands: Vec<Vec<usize>> = Vec::with_capacity(items.len());
        for k in 0..items.len() {
            let mut c = Vec::new();
            for (id, e) in lat.alive() {
                if self.item_matches(rule, &items, k, e) {
                    c.push(id);
                }
            }
            if c.is_empty() {
                return None;
            }
            cands.push(c);
        }
        let n_ctx = rule.contexts.len();
        let mut assign: Vec<usize> = Vec::with_capacity(items.len());
        self.search(ri, &items, n_ctx, &cands, lat, fired, &mut assign)
    }

    #[allow(clippy::too_many_arguments)]
    fn search(
        &mut self,
        ri: usize,
        items: &[u32],
        n_ctx: usize,
        cands: &[Vec<usize>],
        lat: &Lattice,
        fired: &HashSet<(usize, Vec<usize>)>,
        assign: &mut Vec<usize>,
    ) -> Option<Match> {
        let k = assign.len();
        if k == items.len() {
            if fired.contains(&(ri, assign.clone())) {
                return None;
            }
            return self.full_match(ri, items, lat, assign);
        }
        for &id in &cands[k] {
            if assign.contains(&id) {
                continue;
            }
            assign.push(id);
            if self.positions_ok(ri, n_ctx, lat, assign) {
                if let Some(m) = self.search(ri, items, n_ctx, cands, lat, fired, assign) {
                    return Some(m);
                }
            }
            assign.pop();
        }
        None
    }

    /// Check the positional constraints whose items are all assigned.
    fn positions_ok(&self, ri: usize, n_ctx: usize, lat: &Lattice, assign: &[usize]) -> bool {
        let span = |r: Ref| -> Option<(u32, u32)> {
            let idx = match r {
                Ref::Context(i) => i,
                Ref::Input(i) => n_ctx + i,
                Ref::Start => {
                    let v = lat.first_vertex()?;
                    return Some((v, v));
                }
                Ref::End => {
                    let v = lat.last_vertex()?;
                    return Some((v, v));
                }
                Ref::Output(_) => return None,
            };
            let id = *assign.get(idx)?;
            let e = &lat.entries[id];
            Some((e.start, e.end))
        };
        self.rules[ri].positions.iter().all(|p| match *p {
            Pos::Adjacent(a, b) => match (span(a), span(b)) {
                (Some(x), Some(y)) => {
                    x.1 == y.0 || (a == Ref::Start && x.0 == y.0) || (b == Ref::End && x.1 == y.1)
                }
                _ => true,
            },
            Pos::Precedes(a, b) => match (span(a), span(b)) {
                (Some(x), Some(y)) => lat.key(x.1) <= lat.key(y.0),
                _ => true,
            },
            Pos::Same(a, b) => match (span(a), span(b)) {
                (Some(x), Some(y)) => x == y,
                _ => true,
            },
        })
    }

    fn full_match(
        &mut self,
        ri: usize,
        items: &[u32],
        lat: &Lattice,
        assign: &[usize],
    ) -> Option<Match> {
        let cons = self.g.constraint_fn();
        let rule = &self.rules[ri];
        let u = &mut self.unifier;
        u.begin();
        let r = u.add(rule.dag.clone());
        for (k, &id) in assign.iter().enumerate() {
            let h = u.add(lat.entries[id].dag.clone());
            if !u.unify(r + items[k], h, &self.g.ts, &cons) {
                return None;
            }
        }
        if !self.regexes_resolved(rule, r, items, |_| true) {
            return None;
        }
        // Capture groups for template substitution, keyed by item and path.
        let mut captures: HashMap<(usize, Vec<FeatId>), Vec<Option<String>>> = HashMap::new();
        for slot in &rule.regex_slots {
            let h = self.unifier.follow(r + items[slot.item], &slot.path)?;
            let s = self.g.ts.literal_value(self.unifier.node_type(h))?.1;
            let regex_node = rule.dag.follow(items[slot.item], &slot.path)?;
            let re = self.g.ts.regex(rule.dag.ty(regex_node))?;
            let groups = re
                .captures(&s[..])
                .ok()
                .flatten()
                .map(|c| {
                    (0..c.len())
                        .map(|i| c.get(i).map(|m| m.as_str().to_string()))
                        .collect()
                })
                .unwrap_or_else(|| vec![Some(s.to_string())]);
            captures.insert((slot.item, slot.path.clone()), groups);
        }
        let mut outputs = Vec::new();
        for &o in &rule.outputs {
            let mut d = self.unifier.copy(r + o, &[])?;
            self.substitute(&mut d, rule.contexts.len(), &captures, r, items)?;
            outputs.push(d);
        }
        Some(Match {
            ids: assign.to_vec(),
            outputs,
        })
    }

    /// Replace `${...}` references in the string values of an output.
    fn substitute(
        &self,
        d: &mut Dag,
        n_ctx: usize,
        captures: &HashMap<(usize, Vec<FeatId>), Vec<Option<String>>>,
        root: u32,
        items: &[u32],
    ) -> Option<()> {
        for i in 0..d.nodes.len() {
            let t = d.nodes[i].ty;
            let Some((LiteralKind::Str, s)) = self.g.ts.literal_value(t) else {
                continue;
            };
            if !s.contains("${") {
                continue;
            }
            let mut out = String::new();
            let mut rest: &str = &s;
            while let Some(p) = rest.find("${") {
                out.push_str(&rest[..p]);
                let end = rest[p..].find('}')? + p;
                let expr = &rest[p + 2..end];
                out.push_str(&self.eval_ref(expr, n_ctx, captures, root, items)?);
                rest = &rest[end + 1..];
            }
            out.push_str(rest);
            d.nodes[i].ty = self.g.ts.string_literal(&out);
        }
        Some(())
    }

    fn eval_ref(
        &self,
        expr: &str,
        n_ctx: usize,
        captures: &HashMap<(usize, Vec<FeatId>), Vec<Option<String>>>,
        root: u32,
        items: &[u32],
    ) -> Option<String> {
        let (case, inner) =
            if let Some(x) = expr.strip_prefix("lc(").and_then(|x| x.strip_suffix(')')) {
                (Some(false), x)
            } else if let Some(x) = expr.strip_prefix("uc(").and_then(|x| x.strip_suffix(')')) {
                (Some(true), x)
            } else {
                (None, expr)
            };
        let mut parts = inner.split(':');
        let item = parts.next()?;
        let path = parts.next()?;
        let group: usize = parts.next().unwrap_or("0").parse().ok()?;
        let k = match parse_ref(item).ok()? {
            Ref::Context(i) => i,
            Ref::Input(i) => n_ctx + i,
            _ => return None,
        };
        let path = self.g.path(path)?;
        let value = match captures.get(&(k, path.clone())) {
            Some(groups) => groups.get(group).cloned().flatten().unwrap_or_default(),
            None => {
                let h = self.unifier.follow(root + items[k], &path)?;
                self.g
                    .ts
                    .literal_value(self.unifier.node_type(h))?
                    .1
                    .to_string()
            }
        };
        Some(match case {
            Some(true) => value.to_uppercase(),
            Some(false) => value.to_lowercase(),
            None => value,
        })
    }

    /// Apply a match to the lattice. Returns whether the lattice changed.
    fn fire(&mut self, ri: usize, lat: &mut Lattice, m: Match) -> bool {
        let rule = &self.rules[ri];
        let n_ctx = rule.contexts.len();
        let spans = match self.output_spans(rule, n_ctx, lat, &m.ids) {
            Some(s) => s,
            None => return false,
        };
        let inputs: Vec<usize> = m.ids[n_ctx..].to_vec();
        let same =
            |e: &Entry, o: &Dag, span: (u32, u32)| (e.start, e.end) == span && dag_eq(o, &e.dag);
        // Inputs reproduced by some output stay; other inputs are removed.
        let removed: Vec<usize> = inputs
            .iter()
            .copied()
            .filter(|&i| {
                !m.outputs
                    .iter()
                    .zip(&spans)
                    .any(|(o, s)| same(&lat.entries[i], o, *s))
            })
            .collect();
        // Outputs already present in the lattice are not added again.
        let added: Vec<(Dag, (u32, u32))> = m
            .outputs
            .into_iter()
            .zip(spans)
            .filter(|(o, s)| !lat.alive().any(|(_, e)| same(e, o, *s)))
            .collect();
        if removed.is_empty() && added.is_empty() {
            return false;
        }
        for i in removed {
            lat.entries[i].alive = false;
        }
        for (o, (start, end)) in added {
            lat.add(Arc::new(o), start, end);
        }
        true
    }

    /// Spans of the outputs of a match, creating vertices for splits.
    fn output_spans(
        &self,
        rule: &MapRule,
        n_ctx: usize,
        lat: &mut Lattice,
        ids: &[usize],
    ) -> Option<Vec<(u32, u32)>> {
        let n_out = rule.outputs.len();
        let item_span = |r: Ref, lat: &Lattice| -> Option<(u32, u32)> {
            let idx = match r {
                Ref::Context(i) => i,
                Ref::Input(i) => n_ctx + i,
                _ => return None,
            };
            let e = &lat.entries[*ids.get(idx)?];
            Some((e.start, e.end))
        };
        // Items each output is `@`-related to, and `<` links between outputs.
        let mut related: Vec<Vec<Ref>> = vec![Vec::new(); n_out];
        let mut next: Vec<Option<usize>> = vec![None; n_out];
        let mut has_prev = vec![false; n_out];
        for p in &rule.positions {
            match *p {
                Pos::Same(Ref::Output(o), x) | Pos::Same(x, Ref::Output(o)) => {
                    if !matches!(x, Ref::Output(_)) && o < n_out {
                        related[o].push(x);
                    }
                }
                Pos::Adjacent(Ref::Output(a), Ref::Output(b)) if a < n_out && b < n_out => {
                    next[a] = Some(b);
                    has_prev[b] = true;
                }
                _ => {}
            }
        }
        let cover = |refs: &[Ref], lat: &Lattice| -> Option<(u32, u32)> {
            let spans: Vec<(u32, u32)> = refs.iter().filter_map(|&r| item_span(r, lat)).collect();
            let start = spans
                .iter()
                .map(|s| s.0)
                .min_by(|a, b| lat.key(*a).total_cmp(&lat.key(*b)))?;
            let end = spans
                .iter()
                .map(|s| s.1)
                .max_by(|a, b| lat.key(*a).total_cmp(&lat.key(*b)))?;
            Some((start, end))
        };
        let mut spans: Vec<Option<(u32, u32)>> = vec![None; n_out];
        for o in 0..n_out {
            if has_prev[o] || spans[o].is_some() {
                continue;
            }
            // Collect the chain starting at o.
            let mut chain = vec![o];
            while let Some(n) = next[*chain.last().unwrap()] {
                if chain.contains(&n) {
                    break;
                }
                chain.push(n);
            }
            if chain.len() == 1 {
                spans[o] = Some(cover(&related[o], lat)?);
                continue;
            }
            // A chain of outputs splits the span its members are related to.
            let refs: Vec<Ref> = chain.iter().flat_map(|&c| related[c].clone()).collect();
            let (start, end) = cover(&refs, lat)?;
            let (ks, ke) = (lat.key(start), lat.key(end));
            let mut prev = start;
            for (i, &c) in chain.iter().enumerate() {
                let v = if i + 1 == chain.len() {
                    end
                } else {
                    lat.add_vertex(ks + (ke - ks) * (i + 1) as f64 / chain.len() as f64)
                };
                spans[c] = Some((prev, v));
                prev = v;
            }
        }
        spans.into_iter().collect()
    }
}

/// Collect and compile the chart-mapping rules among a grammar's instances
/// with the given status, in definition order.
pub fn compile_rules(
    g: &Grammar,
    status: &str,
    u: &mut Unifier,
) -> Result<Vec<MapRule>, MapRuleError> {
    let paths = MapPaths::erg(g)
        .ok_or_else(|| MapRuleError("grammar lacks chart-mapping features".into()))?;
    let mut rules = Vec::new();
    for inst in g
        .instances
        .iter()
        .filter(|i| i.status.as_deref() == Some(status))
    {
        let dag = g
            .expand(&inst.body, u)
            .map_err(|e| MapRuleError(format!("{}: {e}", inst.name)))?;
        rules.push(MapRule::compile(g, &inst.name, Arc::new(dag), &paths)?);
    }
    Ok(rules)
}

/// Render the string at `path` of a DAG, if it is a string literal.
pub fn string_at(ts: &TypeSystem, dag: &Dag, path: &[FeatId]) -> Option<String> {
    let n = dag.follow(0, path)?;
    match ts.literal_value(dag.ty(n))? {
        (LiteralKind::Str, s) => Some(s.to_string()),
        _ => None,
    }
}
