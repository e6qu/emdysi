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
use emdysi_hpsg::parser::{Deriv, Dtr, EdgeKind, Rule, Scorer};
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

/// Rules, lexical entries and lexical types of emdysi's grammar extensions
/// (`grammar/emdysi`) that stand where an ERG one does, with the same
/// daughters: the ranker scores them under the ERG's name, so that a formal
/// counterpart of an informal ERG analysis ranks like it, and an
/// evaluation against the ERG's gold trees compares them as the same.
pub const EQUIVALENT: &[(&str, &str)] = &[
    ("cl_disc-conj_c", "cl_cnj-frg_c"),
    ("flr-hd_nwh-nc-adj_c", "flr-hd_nwh-nc_c"),
    ("flr-hd_nwh-nc-adj-nmc_c", "flr-hd_nwh-nc-nmc_c"),
    ("comma_adj_pct", "comma_inf_pct"),
    ("pt_-_comma-adj_le", "pt_-_comma-informal_le"),
    ("comma_dom_pct", "comma_inf_pct"),
    ("pt_-_comma-dom_le", "pt_-_comma-informal_le"),
];

/// The ERG's name for `name` (see [`EQUIVALENT`]).
pub fn erg_name(name: &str) -> &str {
    EQUIVALENT
        .iter()
        .find(|(ours, _)| *ours == name)
        .map_or(name, |(_, erg)| erg)
}

fn node_name(g: &Grammar, rules: &[Rule], d: &Deriv) -> String {
    match &d.kind {
        EdgeKind::Lex { inst, .. } => erg_name(&lexical_type(g, *inst)).to_string(),
        EdgeKind::Rule(ri) => erg_name(&rules[*ri].name).to_string(),
        EdgeKind::Cover => "fragment".to_string(),
    }
}

/// Feature strings of a derivation rooted with root condition `root`.
pub fn features(g: &Grammar, rules: &[Rule], d: &Deriv, root: &str) -> Vec<String> {
    // British-spelling twins of root conditions share their weights.
    let root = root.strip_suffix("_br").unwrap_or(root);
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
            out.push(format!("lex:{}", erg_name(&g.instances[*inst].name)));
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
        EdgeKind::Cover => {
            for k in &d.daughters {
                collect(g, rules, k, "^", out);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Model {
    pub weights: HashMap<String, f64>,
    /// Scores divided by this give calibrated log-probabilities: the
    /// probability of a reading among the readings found is proportional
    /// to `exp(score / temperature)` (fitted on held-out items; see
    /// [`fit_temperature`]).
    pub temperature: f64,
}

impl Default for Model {
    fn default() -> Model {
        Model {
            weights: HashMap::new(),
            temperature: 1.0,
        }
    }
}

impl Model {
    /// Parse a model file: one `weight<TAB>feature` per line.
    pub fn parse(src: &str) -> Model {
        let temperature = src
            .lines()
            .find_map(|l| l.strip_prefix("# temperature\t")?.trim().parse().ok())
            .unwrap_or(1.0);
        let weights = src
            .lines()
            .filter_map(|l| {
                let (w, f) = l.split_once('\t')?;
                Some((f.to_string(), w.parse().ok()?))
            })
            .collect();
        Model {
            weights,
            temperature,
        }
    }

    pub fn to_tsv(&self) -> String {
        let mut v: Vec<(&String, &f64)> = self
            .weights
            .iter()
            .filter(|(_, w)| w.abs() > 1e-9)
            .collect();
        v.sort_by(|a, b| a.0.cmp(b.0));
        let head = if self.temperature == 1.0 {
            String::new()
        } else {
            format!("# temperature\t{:.4}\n", self.temperature)
        };
        head + &v
            .iter()
            .map(|(f, w)| format!("{w:.6}\t{f}\n"))
            .collect::<String>()
    }

    pub fn score(&self, feats: &[String]) -> f64 {
        feats.iter().filter_map(|f| self.weights.get(f)).sum()
    }
}

/// The model's local features as a chart-pruning scorer: lexical entries
/// and types for words, rules with their daughters' categories for phrases.
/// Features that depend on the parent node are left to the final ranking.
pub struct ChartScorer<'a> {
    pub grammar: &'a Grammar,
    pub rules: &'a [Rule],
    pub model: &'a Model,
    /// Lexical type of each instance (see [`lexical_type`]).
    pub le_types: &'a [String],
}

impl ChartScorer<'_> {
    fn weight(&self, f: &str) -> f64 {
        self.model.weights.get(f).copied().unwrap_or(0.0)
    }

    fn category(&self, d: &Dtr) -> String {
        match *d {
            Dtr::Lex(inst) => erg_name(&self.le_types[inst]).to_string(),
            Dtr::Rule(ri) => self
                .rules
                .get(ri)
                .map_or_else(|| "fragment".to_string(), |r| erg_name(&r.name).to_string()),
        }
    }
}

impl Scorer for ChartScorer<'_> {
    fn lexical(&self, inst: usize) -> f64 {
        let le = erg_name(&self.le_types[inst]);
        self.weight(&format!(
            "lex:{}",
            erg_name(&self.grammar.instances[inst].name)
        )) + self.weight(&format!("le:{le}"))
    }

    fn rule(&self, rule: usize, dtrs: &[Dtr]) -> f64 {
        let name = erg_name(&self.rules[rule].name);
        let kids: Vec<String> = dtrs.iter().map(|d| self.category(d)).collect();
        self.weight(&format!("r:{name}")) + self.weight(&format!("rd:{name}>{}", kids.join(",")))
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
    Model {
        weights,
        temperature: 1.0,
    }
}

/// Train a conditional log-linear model (maximum entropy, as for the
/// Redwoods rankers): maximise the log-probability of the gold reading
/// among each item's readings, with a Gaussian prior (L2 penalty `l2`) on
/// the weights, by full-batch gradient ascent with Adam for `iterations`
/// steps. Readings with the same features as the gold one count as gold.
/// More stable than the perceptron when the training items change.
pub fn train_maxent(examples: &[Example], l2: f64, iterations: usize) -> Model {
    let mut ids: HashMap<&str, u32> = HashMap::new();
    let mut names: Vec<&str> = Vec::new();
    // Each item: readings as feature ids (repeats kept), and which are gold.
    let mut items: Vec<(Vec<Vec<u32>>, Vec<bool>)> = Vec::new();
    for ex in examples {
        if ex.readings.len() < 2 {
            continue;
        }
        let mut gold_set = ex.readings[ex.gold].clone();
        gold_set.sort();
        let readings: Vec<Vec<u32>> = ex
            .readings
            .iter()
            .map(|r| {
                r.iter()
                    .map(|f| {
                        *ids.entry(f.as_str()).or_insert_with(|| {
                            names.push(f.as_str());
                            (names.len() - 1) as u32
                        })
                    })
                    .collect()
            })
            .collect();
        let gold: Vec<bool> = ex
            .readings
            .iter()
            .map(|r| {
                let mut s = r.clone();
                s.sort();
                s == gold_set
            })
            .collect();
        if gold.iter().all(|&g| g) {
            continue;
        }
        items.push((readings, gold));
    }
    let n = names.len();
    let mut w = vec![0.0f64; n];
    let (mut m, mut v) = (vec![0.0f64; n], vec![0.0f64; n]);
    let (b1, b2, rate, eps) = (0.9f64, 0.999f64, 0.05f64, 1e-8f64);
    let mut grad = vec![0.0f64; n];
    for step in 1..=iterations {
        grad.iter_mut().zip(&w).for_each(|(g, wi)| *g = -l2 * wi);
        for (readings, gold) in &items {
            let scores: Vec<f64> = readings
                .iter()
                .map(|r| r.iter().map(|&f| w[f as usize]).sum())
                .collect();
            let top = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let e: Vec<f64> = scores.iter().map(|s| (s - top).exp()).collect();
            let z: f64 = e.iter().sum();
            let zg: f64 = e
                .iter()
                .zip(gold)
                .filter(|(_, g)| **g)
                .map(|(x, _)| x)
                .sum();
            // d/dw log(Zgold/Z) = E_gold[f] - E_all[f]
            for (k, r) in readings.iter().enumerate() {
                let c = if gold[k] { e[k] / zg } else { 0.0 } - e[k] / z;
                if c.abs() < 1e-12 {
                    continue;
                }
                for &f in r {
                    grad[f as usize] += c;
                }
            }
        }
        for i in 0..n {
            m[i] = b1 * m[i] + (1.0 - b1) * grad[i];
            v[i] = b2 * v[i] + (1.0 - b2) * grad[i] * grad[i];
            let mh = m[i] / (1.0 - b1.powi(step as i32));
            let vh = v[i] / (1.0 - b2.powi(step as i32));
            w[i] += rate * mh / (vh.sqrt() + eps);
        }
    }
    let weights = names
        .iter()
        .zip(&w)
        .filter(|(_, wi)| wi.abs() > 1e-6)
        .map(|(f, wi)| (f.to_string(), *wi))
        .collect();
    Model {
        weights,
        temperature: 1.0,
    }
}

/// The temperature that makes `model`'s scores calibrated probabilities on
/// `examples` (items not trained on): the one, on a log grid from 0.01 to
/// 100, that maximizes the mean log-probability of the gold reading under
/// `exp(score / t)`, normalized over each item's readings. Readings with
/// the gold reading's features count as gold. Returns the temperature and
/// the mean log-probability.
pub fn fit_temperature(model: &Model, examples: &[Example]) -> (f64, f64) {
    let scored: Vec<(Vec<f64>, Vec<bool>)> = examples
        .iter()
        .map(|e| {
            let gold = &e.readings[e.gold];
            (
                e.readings.iter().map(|r| model.score(r)).collect(),
                e.readings.iter().map(|r| r == gold).collect(),
            )
        })
        .collect();
    let loglik = |t: f64| -> f64 {
        let mut sum = 0.0;
        for (s, g) in &scored {
            let max = s.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let z: f64 = s.iter().map(|x| ((x - max) / t).exp()).sum();
            let gz: f64 = s
                .iter()
                .zip(g)
                .filter(|(_, g)| **g)
                .map(|(x, _)| ((x - max) / t).exp())
                .sum();
            sum += (gz / z).ln();
        }
        sum / scored.len().max(1) as f64
    };
    (0..=400)
        .map(|k| 10f64.powf(-2.0 + k as f64 / 100.0))
        .map(|t| (t, loglik(t)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap_or((1.0, 0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maxent_prefers_gold_features() {
        let f = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let examples = vec![
            Example {
                readings: vec![f(&["a", "x"]), f(&["b", "x"])],
                gold: 0,
            },
            Example {
                readings: vec![f(&["b", "y"]), f(&["a", "y"]), f(&["b", "z"])],
                gold: 1,
            },
            // Every reading is gold: nothing to learn.
            Example {
                readings: vec![f(&["c"]), f(&["c"])],
                gold: 0,
            },
        ];
        let m = train_maxent(&examples, 1.0, 200);
        assert!(m.score(&f(&["a"])) > m.score(&f(&["b"])));
        assert!(!m.weights.contains_key("c"));
    }
}
