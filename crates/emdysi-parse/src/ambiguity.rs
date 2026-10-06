//! Ambiguity: the readings of a sentence grouped by what they mean.
//!
//! Many readings differ only in how the grammar derives the same meaning.
//! Two readings mean the same when their semantics (MRS) have the same
//! predicate-argument dependencies: each predicate with the words it takes
//! as arguments. Each group of readings is an *interpretation*, with the
//! probability mass of its readings under the ranking model (a log-linear
//! model, its scores calibrated by a temperature fitted on held-out gold
//! items, so the probability of a reading is proportional to
//! `exp(score / temperature)` among the readings found). A sentence is ambiguous when more than one
//! interpretation keeps a share of the mass: *I saw the man with the
//! telescope* (*with* modifies *saw* or *man*), *Time flies like an arrow*.

use std::collections::{BTreeMap, BTreeSet};

use emdysi_hpsg::mrs::{Ep, Mrs};

use crate::Reading;

/// One meaning of a sentence: readings with the same dependencies.
#[derive(Debug, Clone)]
pub struct Interpretation {
    /// Indices of the readings (best first).
    pub readings: Vec<usize>,
    /// Share of the probability mass of all readings found.
    pub probability: f64,
    /// Dependencies, e.g. `with(saw, telescope)`.
    pub dependencies: BTreeSet<String>,
}

impl Interpretation {
    /// Dependencies of this interpretation that `other` lacks.
    pub fn differences(&self, other: &Interpretation) -> Vec<String> {
        self.dependencies
            .difference(&other.dependencies)
            .cloned()
            .collect()
    }
}

/// Interpretations other than the most probable one with at least this
/// share of the probability mass make a sentence ambiguous.
pub const MIN_SHARE: f64 = 0.05;

/// The interpretations of `readings` (ranked best first, as in a
/// [`crate::Parse`]) of `sentence`, most probable first. Readings without
/// semantics, and readings that take a lower-case word for a name, are
/// left out.
pub fn interpretations(
    readings: &[Reading],
    temperature: f64,
    sentence: &str,
) -> Vec<Interpretation> {
    let chars: Vec<char> = sentence.chars().collect();
    // A name for a word written in lower case ("Apple" for *apples*): the
    // spelling rules it out.
    let lower_name = |m: &Mrs| {
        m.eps.iter().any(|e| {
            e.pred == "named"
                && e.lnk
                    .and_then(|(f, _)| chars.get(f))
                    .is_some_and(|c| c.is_lowercase())
        })
    };
    let kept: Vec<(usize, &Reading, &Mrs)> = readings
        .iter()
        .enumerate()
        .filter_map(|(i, r)| Some((i, r, r.mrs.as_ref()?)))
        .filter(|(_, _, m)| !lower_name(m))
        .collect();
    let best = kept
        .iter()
        .map(|(_, r, _)| r.score)
        .fold(f64::NEG_INFINITY, f64::max);
    let weight = |r: &Reading| ((r.score - best) / temperature).exp();
    let total: f64 = kept.iter().map(|(_, r, _)| weight(r)).sum();
    let mut groups: BTreeMap<BTreeSet<String>, Interpretation> = BTreeMap::new();
    for (i, r, m) in kept {
        let deps = dependencies(m, &chars);
        let g = groups
            .entry(deps.clone())
            .or_insert_with(|| Interpretation {
                readings: Vec::new(),
                probability: 0.0,
                dependencies: deps,
            });
        g.readings.push(i);
        g.probability += weight(r) / total;
    }
    let mut out: Vec<Interpretation> = groups.into_values().collect();
    out.sort_by(|a, b| {
        b.probability
            .total_cmp(&a.probability)
            .then(a.readings[0].cmp(&b.readings[0]))
    });
    out
}

/// How an EP is shown: the words it covers for a word's predicate
/// (`telescope`), else the predicate's name (`compound`).
fn name(ep: &Ep, chars: &[char]) -> String {
    let word = ep
        .lnk
        .and_then(|(f, t)| chars.get(f..t))
        .map(|c| {
            c.iter()
                .collect::<String>()
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|w| !w.is_empty() && !w.contains(' '));
    match word {
        Some(w) if ep.pred.starts_with('_') => w,
        _ => ep
            .pred
            .trim_start_matches('_')
            .trim_end_matches("_rel")
            .to_string(),
    }
}

/// The predicate-argument dependencies of an MRS, as strings
/// `head(arg1, arg2, ...)` over the words of the sentence. Quantifiers and
/// scope are left out: they are underspecified in the MRS anyway.
pub fn dependencies(m: &Mrs, chars: &[char]) -> BTreeSet<String> {
    // The EP that a variable stands for: the non-quantifier whose ARG0 it
    // is; a handle stands for the EPs with that label (through a qeq).
    let by_arg0: BTreeMap<&str, &Ep> = m
        .eps
        .iter()
        .filter(|e| !e.is_quantifier())
        .filter_map(|e| Some((e.arg0()?, e)))
        .collect();
    let qeq: BTreeMap<&str, &str> = m
        .hcons
        .iter()
        .map(|(h, _, l)| (h.as_str(), l.as_str()))
        .collect();
    let target = |v: &str| -> Option<String> {
        if let Some(e) = by_arg0.get(v) {
            return Some(name(e, chars));
        }
        let label = qeq.get(v).copied().unwrap_or(v);
        let mut heads: Vec<String> = m
            .eps
            .iter()
            .filter(|e| e.label == label && !e.is_quantifier())
            // The head of a scopal argument: an EP of the label that no
            // other EP of the label takes as an argument.
            .filter(|e| {
                let a0 = e.arg0();
                !m.eps.iter().any(|o| {
                    o.label == label
                        && !std::ptr::eq(*e, o)
                        && o.args
                            .iter()
                            .any(|(r, x)| r != "ARG0" && Some(x.as_str()) == a0)
                })
            })
            .map(|e| name(e, chars))
            .collect();
        heads.sort();
        heads.dedup();
        (!heads.is_empty()).then(|| heads.join("+"))
    };
    let mut out = BTreeSet::new();
    for e in m.eps.iter().filter(|e| !e.is_quantifier()) {
        let args: Vec<String> = e
            .args
            .iter()
            .filter(|(r, _)| r != "ARG0")
            .map(|(r, v)| {
                let t = target(v).unwrap_or_else(|| "_".into());
                if r.starts_with("ARG") {
                    t
                } else {
                    format!("{}={t}", r.to_lowercase())
                }
            })
            .collect();
        let mut s = name(e, chars);
        if let Some(c) = &e.carg {
            s = format!("{s}:{c}");
        }
        out.insert(format!("{s}({})", args.join(", ")));
    }
    out
}
