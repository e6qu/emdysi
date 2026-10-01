//! Parse ranking with a linear model over derivation-tree features.
//!
//! Features are local configurations in the style of the Redwoods ranking
//! models: a rule with the categories of its daughters, the same with the
//! parent rule, lexical entries and their lexical types, and the root. The
//! weights are trained with an averaged perceptron on the gold derivations
//! of the vendored test suites, so the model contains nothing derived from
//! data with incompatible licenses.

use std::collections::HashMap;

use emdysi_hpsg::Grammar;
use emdysi_hpsg::parser::{Deriv, EdgeKind, Rule};
use emdysi_tdl::Term;

/// The lexical type of an instance: the first type in its definition.
pub fn lexical_type(g: &Grammar, inst: usize) -> String {
    g.instances[inst]
        .body
        .0
        .iter()
        .find_map(|t| match t {
            Term::Type(n) => Some(n.to_lowercase()),
            _ => None,
        })
        .unwrap_or_default()
}

fn node_name(g: &Grammar, rules: &[Rule], d: &Deriv) -> String {
    match &d.kind {
        EdgeKind::Lex { inst, .. } => lexical_type(g, *inst),
        EdgeKind::Rule(ri) => rules[*ri].name.clone(),
    }
}

/// Feature strings of a derivation rooted with root condition `root`.
pub fn features(g: &Grammar, rules: &[Rule], d: &Deriv, root: &str) -> Vec<String> {
    let mut out = vec![
        format!("root:{root}"),
        format!("top:{root}>{}", node_name(g, rules, d)),
    ];
    collect(g, rules, d, "^", &mut out);
    out
}

fn collect(g: &Grammar, rules: &[Rule], d: &Deriv, parent: &str, out: &mut Vec<String>) {
    let name = node_name(g, rules, d);
    match &d.kind {
        EdgeKind::Lex { inst, .. } => {
            out.push(format!("lex:{}", g.instances[*inst].name));
            out.push(format!("le:{name}"));
            out.push(format!("le^:{parent}>{name}"));
        }
        EdgeKind::Rule(_) => {
            let kids: Vec<String> = d.daughters.iter().map(|k| node_name(g, rules, k)).collect();
            let kids = kids.join(",");
            out.push(format!("r:{name}"));
            out.push(format!("rd:{name}>{kids}"));
            out.push(format!("prd:{parent}^{name}>{kids}"));
            for k in &d.daughters {
                collect(g, rules, k, &name, out);
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Model {
    pub weights: HashMap<String, f64>,
}

impl Model {
    /// Parse a model file: one `weight<TAB>feature` per line.
    pub fn parse(src: &str) -> Model {
        let weights = src
            .lines()
            .filter_map(|l| {
                let (w, f) = l.split_once('\t')?;
                Some((f.to_string(), w.parse().ok()?))
            })
            .collect();
        Model { weights }
    }

    pub fn to_tsv(&self) -> String {
        let mut v: Vec<(&String, &f64)> = self
            .weights
            .iter()
            .filter(|(_, w)| w.abs() > 1e-9)
            .collect();
        v.sort_by(|a, b| a.0.cmp(b.0));
        v.iter().map(|(f, w)| format!("{w:.6}\t{f}\n")).collect()
    }

    pub fn score(&self, feats: &[String]) -> f64 {
        feats.iter().filter_map(|f| self.weights.get(f)).sum()
    }
}

/// One training instance: the feature lists of all readings and the index
/// of the gold reading.
pub struct Example {
    pub readings: Vec<Vec<String>>,
    pub gold: usize,
}

/// Train an averaged perceptron.
pub fn train(examples: &[Example], epochs: usize) -> Model {
    let mut w: HashMap<String, f64> = HashMap::new();
    // Sum of weights over time, for averaging (lazy update trick).
    let mut total: HashMap<String, f64> = HashMap::new();
    let mut stamp: HashMap<String, usize> = HashMap::new();
    let mut t = 0usize;
    let score =
        |w: &HashMap<String, f64>, f: &[String]| -> f64 { f.iter().filter_map(|x| w.get(x)).sum() };
    for _ in 0..epochs {
        for ex in examples {
            t += 1;
            if ex.readings.len() < 2 {
                continue;
            }
            let best = (0..ex.readings.len())
                .max_by(|&a, &b| score(&w, &ex.readings[a]).total_cmp(&score(&w, &ex.readings[b])))
                .unwrap();
            if best == ex.gold || ex.readings[best] == ex.readings[ex.gold] {
                continue;
            }
            let mut delta: HashMap<&String, f64> = HashMap::new();
            for f in &ex.readings[ex.gold] {
                *delta.entry(f).or_default() += 1.0;
            }
            for f in &ex.readings[best] {
                *delta.entry(f).or_default() -= 1.0;
            }
            for (f, d) in delta {
                if d == 0.0 {
                    continue;
                }
                let cur = w.get(f).copied().unwrap_or(0.0);
                let last = stamp.get(f).copied().unwrap_or(0);
                *total.entry(f.clone()).or_default() += cur * (t - last) as f64;
                stamp.insert(f.clone(), t);
                w.insert(f.clone(), cur + d);
            }
        }
    }
    for (f, cur) in &w {
        let last = stamp.get(f).copied().unwrap_or(0);
        *total.entry(f.clone()).or_default() += cur * (t - last) as f64;
    }
    let weights = total
        .into_iter()
        .map(|(f, s)| (f, s / t.max(1) as f64))
        .collect();
    Model { weights }
}
