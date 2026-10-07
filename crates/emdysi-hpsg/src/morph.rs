//! Orthographic morphology: undoing the `%suffix`/`%prefix` patterns of
//! lexical rules and consulting the irregular-forms table.
//!
//! A pattern pair `(lhs rhs)` rewrites the end (or start) of a stem matching
//! `lhs` into `rhs`. Analysis runs it backwards: a surface form ending in
//! `rhs` may come from the stem ending in `lhs`. `!x` variables match one
//! character of letter set `x`, and repeated variables must match the same
//! character, as in `(!c !c!ced)` for *stopped*.

use std::collections::{HashMap, HashSet};

use emdysi_tdl::PatElem;

use crate::grammar::Grammar;

#[derive(Debug, Clone)]
pub struct OrthRule {
    /// Instance name of the lexical rule.
    pub name: String,
    pub prefix: bool,
    pub pairs: Vec<(Vec<PatElem>, Vec<PatElem>)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Analysis {
    pub stem: String,
    /// Orthographic rules to apply to the stem, innermost first.
    pub rules: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Morphology {
    pub rules: Vec<OrthRule>,
    letter_sets: HashMap<char, HashSet<char>>,
    /// Surface form -> (rule, stem), from the irregular-forms table.
    irregs: HashMap<String, Vec<(String, String)>>,
    /// (rule, stem) pairs that have an irregular form: the regular
    /// application of the rule to the stem is blocked (as in ACE and the
    /// LKB), so that e.g. *buyed* is not a past tense of *buy*.
    irregular: HashSet<(String, String)>,
}

impl Morphology {
    pub fn from_grammar(g: &Grammar) -> Morphology {
        let rules = g
            .instances
            .iter()
            .filter_map(|i| {
                i.affix.as_ref().map(|a| OrthRule {
                    name: i.name.clone(),
                    prefix: a.prefix,
                    pairs: a.pairs.clone(),
                })
            })
            .collect();
        let letter_sets = g
            .letter_sets
            .iter()
            .map(|ls| (ls.var, ls.chars.iter().copied().collect()))
            .collect();
        Morphology {
            rules,
            letter_sets,
            irregs: HashMap::new(),
            irregular: HashSet::new(),
        }
    }

    /// Read an irregular-forms table: lines of `form RULE stem`, optionally
    /// wrapped in a Lisp string (a lone `"` on the first and last lines).
    pub fn load_irregs(&mut self, src: &str) {
        for line in src.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let [form, rule, stem] = parts[..] {
                self.irregs
                    .entry(form.to_lowercase())
                    .or_default()
                    .push((rule.to_lowercase(), stem.to_lowercase()));
                self.irregular
                    .insert((rule.to_lowercase(), stem.to_lowercase()));
            }
        }
    }

    /// The irregular forms of a stem (*bought* for *buy*), from the
    /// irregular-forms table.
    pub fn irregular_forms(&self, stem: &str) -> Vec<String> {
        let stem = stem.to_lowercase();
        let mut out: Vec<String> = self
            .irregs
            .iter()
            .filter(|(_, v)| v.iter().any(|(_, s)| *s == stem))
            .map(|(f, _)| f.clone())
            .collect();
        out.sort();
        out
    }

    /// The irregular forms of a stem made by one of `rules` (*bought* for
    /// *buy* and `v_pst_olr`).
    pub fn irregular_forms_by(&self, stem: &str, rules: &[&str]) -> Vec<String> {
        let stem = stem.to_lowercase();
        let mut out: Vec<String> = self
            .irregs
            .iter()
            .filter(|(_, v)| {
                v.iter()
                    .any(|(r, s)| *s == stem && rules.contains(&r.as_str()))
            })
            .map(|(f, _)| f.clone())
            .collect();
        out.sort();
        out
    }

    pub fn is_orth_rule(&self, name: &str) -> bool {
        self.rules.iter().any(|r| r.name == name)
    }

    /// All analyses of `form` with at most `max_rules` orthographic rules
    /// whose stem satisfies `is_stem`. The form itself is an analysis with
    /// no rules when it is a stem.
    pub fn analyze(
        &self,
        form: &str,
        is_stem: &dyn Fn(&str) -> bool,
        max_rules: usize,
    ) -> Vec<Analysis> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        self.analyze_into(
            form,
            Vec::new(),
            false,
            is_stem,
            max_rules,
            &mut out,
            &mut seen,
        );
        out
    }

    /// `identity`: an irregular form equal to its stem (*set*, past of
    /// *set*) was already undone. Such steps leave the form unchanged, so
    /// they could stack without end (*set* as the past of the past of
    /// *set*); one per analysis is enough, as two inflections never
    /// combine.
    #[allow(clippy::too_many_arguments)]
    fn analyze_into(
        &self,
        form: &str,
        outer: Vec<String>,
        identity: bool,
        is_stem: &dyn Fn(&str) -> bool,
        budget: usize,
        out: &mut Vec<Analysis>,
        seen: &mut HashSet<(String, Vec<String>)>,
    ) {
        if !seen.insert((form.to_string(), outer.clone())) {
            return;
        }
        if is_stem(form) {
            let mut rules = outer.clone();
            rules.reverse();
            out.push(Analysis {
                stem: form.to_string(),
                rules,
            });
        }
        if budget == 0 {
            return;
        }
        if let Some(irr) = self.irregs.get(&form.to_lowercase()) {
            let lower = form.to_lowercase();
            for (rule, stem) in irr {
                let same = *stem == lower;
                if same && identity {
                    continue;
                }
                let mut o = outer.clone();
                o.push(rule.clone());
                self.analyze_into(stem, o, identity || same, is_stem, budget - 1, out, seen);
            }
        }
        for rule in &self.rules {
            for (lhs, rhs) in &rule.pairs {
                if let Some(base) = self.undo(form, lhs, rhs, rule.prefix) {
                    if base.is_empty()
                        || self
                            .irregular
                            .contains(&(rule.name.to_lowercase(), base.to_lowercase()))
                    {
                        continue;
                    }
                    let mut o = outer.clone();
                    o.push(rule.name.clone());
                    self.analyze_into(&base, o, identity, is_stem, budget - 1, out, seen);
                }
            }
        }
    }

    /// Undo one pattern pair on `form`, giving the base form.
    fn undo(&self, form: &str, lhs: &[PatElem], rhs: &[PatElem], prefix: bool) -> Option<String> {
        let chars: Vec<char> = form.chars().collect();
        if rhs.len() > chars.len() {
            return None;
        }
        let mut bind: HashMap<char, char> = HashMap::new();
        let window: &[char] = if prefix {
            &chars[..rhs.len()]
        } else {
            &chars[chars.len() - rhs.len()..]
        };
        for (p, &c) in rhs.iter().zip(window) {
            match *p {
                PatElem::Char(x) => {
                    if x != c {
                        return None;
                    }
                }
                PatElem::Var(v) => {
                    if !self.letter_sets.get(&v).is_some_and(|s| s.contains(&c)) {
                        return None;
                    }
                    if *bind.entry(v).or_insert(c) != c {
                        return None;
                    }
                }
            }
        }
        let mut repl = String::new();
        for p in lhs {
            match *p {
                PatElem::Char(x) => repl.push(x),
                PatElem::Var(v) => repl.push(*bind.get(&v)?),
            }
        }
        let rest: String = if prefix {
            chars[rhs.len()..].iter().collect()
        } else {
            chars[..chars.len() - rhs.len()].iter().collect()
        };
        Some(if prefix { repl + &rest } else { rest + &repl })
    }
}
