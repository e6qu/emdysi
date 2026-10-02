//! Guarded rewriting (decision D10).
//!
//! For each sentence with diagnostics, candidate rewrites come from three
//! sources: the automatic fixes of the diagnostics, spelling suggestions
//! (typo correction), and, optionally, a local language model that is told
//! what is wrong with the sentence. Every candidate is then checked by the
//! deterministic layers and is kept only if
//!
//! 1. the grammar gives it a strict analysis,
//! 2. it introduces no diagnostic of a kind the original did not have and
//!    has fewer diagnostics than the original,
//! 3. it keeps the meaning: the content predicates of the original's
//!    semantics (MRS), outside the words a diagnostic flagged, are still
//!    there (all of them for rule and spelling fixes, a configurable share
//!    for model rewrites).
//!
//! Among the candidates that pass, the model's likelihood (mean token
//! log-probability, when a model is loaded) decides, then the order of the
//! candidates (the diagnostics' own fixes, then spelling suggestions in
//! their ranked order, then model rewrites). Candidates that fail are never shown.

use std::collections::{HashMap, HashSet};

use emdysi_check::{Analysis, Checker, Diagnostic, Format, Options, analyze};
use emdysi_parse::Erg;
use emdysi_parse::mrs::Mrs;

#[cfg(feature = "llama")]
pub mod llama;

/// A language model: scores text and generates continuations.
pub trait LanguageModel {
    /// Log-probability (natural log) of `text` following `prefix`, and the
    /// number of tokens of `text`.
    fn logprob(&mut self, prefix: &str, text: &str) -> Option<(f64, usize)>;
    /// Generate a continuation of a chat prompt: `system` instructions and
    /// a `user` message. `temperature` 0 is greedy; `seed` makes sampling
    /// reproducible. Generation stops at the end of a line.
    fn generate(
        &mut self,
        system: &str,
        user: &str,
        max_tokens: usize,
        temperature: f32,
        seed: u32,
    ) -> Option<String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The automatic fixes of the sentence's diagnostics.
    RuleFix,
    /// A spelling suggestion for an unknown word.
    Spelling,
    /// The language model.
    Model,
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub text: String,
    pub source: Source,
    /// Mean token log-probability under the model, if one is loaded.
    pub lm_score: Option<f64>,
    /// Why the candidate was rejected, if it was.
    pub rejected: Option<String>,
}

/// The outcome for one sentence.
#[derive(Debug, Clone)]
pub struct Proposal {
    pub sentence: usize,
    pub original: String,
    /// The diagnostics that prompted the rewrite.
    pub issues: Vec<String>,
    /// The accepted rewrite, if any.
    pub chosen: Option<Candidate>,
    /// Every candidate considered, accepted or not.
    pub candidates: Vec<Candidate>,
}

#[derive(Debug, Clone)]
pub struct RewriteOptions {
    /// Model rewrites sampled per sentence (plus one greedy).
    pub samples: usize,
    pub temperature: f32,
    pub max_tokens: usize,
    /// Share of the original's content predicates a model rewrite must
    /// keep (rule and spelling fixes must keep all).
    pub keep_meaning: f64,
    /// Rules whose diagnostics prompt a rewrite; empty: all.
    pub rules: Vec<String>,
}

impl Default for RewriteOptions {
    fn default() -> Self {
        RewriteOptions {
            samples: 3,
            temperature: 0.7,
            max_tokens: 96,
            keep_meaning: 0.8,
            rules: Vec::new(),
        }
    }
}

const SYSTEM_PROMPT: &str = "You are a careful copy editor. Rewrite the sentence the user gives you so that it fixes the listed problems. Keep the meaning, the facts and the register; change as little as possible; do not add new claims. Answer with the rewritten sentence only, on one line.";

/// Content predicates of an MRS (lexical predicates, which start with `_`,
/// except quantifiers), with the character span of each.
fn content_preds(m: &Mrs) -> Vec<(String, Option<(usize, usize)>)> {
    m.eps
        .iter()
        .filter(|e| e.pred.starts_with('_') && !e.is_quantifier())
        .map(|e| (e.pred.clone(), e.lnk))
        .collect()
}

/// The semantics of a sentence: its best strict reading, or else the best
/// reading of the grammar-error variant (which still analyses the meaning
/// of an erroneous sentence).
fn meaning(s: &emdysi_check::Sentence) -> Option<&Mrs> {
    if s.strict() {
        return s.best().and_then(|r| r.mrs.as_ref());
    }
    s.mal_parse
        .as_ref()
        .and_then(|p| p.readings.first())
        .and_then(|r| r.mrs.as_ref())
}

/// Levenshtein distance in characters.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j] + (ca != cb) as usize)
                .min(prev[j + 1] + 1)
                .min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

pub struct Rewriter<'a> {
    pub erg: &'a Erg,
    pub checker: &'a Checker,
    pub options: Options,
    pub rewrite: RewriteOptions,
    pub model: Option<&'a mut dyn LanguageModel>,
}

impl Rewriter<'_> {
    /// Diagnostics of a single sentence, analysed on its own.
    fn check_one(&self, text: &str) -> (Analysis, Vec<Diagnostic>) {
        let a = analyze(self.erg, text, Format::Plain, &self.options);
        let d = self.checker.check(self.erg, &a);
        (a, d)
    }

    /// Candidate rewrites from the diagnostics' fixes and suggestions, as
    /// whole sentences. `diags` have ranges relative to the sentence.
    fn deterministic(&self, sentence: &str, diags: &[Diagnostic]) -> Vec<Candidate> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        let mut push = |text: String, source: Source, out: &mut Vec<Candidate>| {
            if text != sentence && seen.insert(text.clone()) {
                out.push(Candidate {
                    text,
                    source,
                    lm_score: None,
                    rejected: None,
                });
            }
        };
        let replace = |r: &std::ops::Range<usize>, rep: &str| {
            let mut s = sentence.to_string();
            s.replace_range(r.clone(), rep);
            s
        };
        // All automatic fixes together, then each alone.
        let (all, n) = emdysi_check::apply_fixes(sentence, diags);
        if n > 0 {
            push(all, Source::RuleFix, &mut out);
        }
        for d in diags {
            if let Some(rep) = &d.replacement {
                push(replace(&d.range, rep), Source::RuleFix, &mut out);
            }
            if d.rule.ends_with("spelling") {
                for s in d.suggestions.iter().take(6) {
                    push(replace(&d.range, s), Source::Spelling, &mut out);
                }
            }
        }
        out
    }

    fn model_candidates(&mut self, sentence: &str, issues: &[String]) -> Vec<Candidate> {
        let Some(model) = self.model.as_mut() else {
            return Vec::new();
        };
        let user = format!(
            "Sentence: {sentence}\nProblems:\n{}\nRewritten sentence:",
            issues
                .iter()
                .map(|i| format!("- {i}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        for k in 0..=self.rewrite.samples {
            let t = if k == 0 {
                0.0
            } else {
                self.rewrite.temperature
            };
            let Some(text) =
                model.generate(SYSTEM_PROMPT, &user, self.rewrite.max_tokens, t, k as u32)
            else {
                continue;
            };
            let text = text
                .trim()
                .trim_matches(|c| c == '"' || c == '“' || c == '”')
                .trim()
                .to_string();
            if !text.is_empty() && text != sentence && seen.insert(text.clone()) {
                out.push(Candidate {
                    text,
                    source: Source::Model,
                    lm_score: None,
                    rejected: None,
                });
            }
        }
        out
    }

    /// Check a candidate against the original sentence; `None` if it
    /// passes, else the reason.
    fn verify(
        &self,
        c: &Candidate,
        original: &emdysi_check::Sentence,
        original_diags: &[Diagnostic],
        flagged: &[std::ops::Range<usize>],
    ) -> Option<String> {
        let (a, d) = self.check_one(&c.text);
        if a.sentences.len() != 1 {
            return Some(format!("became {} sentences", a.sentences.len()));
        }
        let s = &a.sentences[0];
        if !s.strict() {
            return Some("the grammar finds no strict analysis".into());
        }
        let before: HashSet<&str> = original_diags.iter().map(|d| d.rule.as_str()).collect();
        if let Some(new) = d.iter().find(|x| !before.contains(x.rule.as_str())) {
            return Some(format!("introduces {}: {}", new.rule, new.message));
        }
        if d.len() >= original_diags.len() {
            return Some("fixes nothing".into());
        }
        // Meaning: content predicates outside the flagged words survive.
        if let (Some(m0), Some(m1)) = (meaning(original), meaning(s)) {
            let chars: Vec<(usize, usize)> = original
                .original
                .char_indices()
                .enumerate()
                .map(|(ci, (b, _))| (ci, b))
                .collect();
            let byte_of = |ci: usize| chars.get(ci).map_or(original.original.len(), |c| c.1);
            let keep: Vec<String> = content_preds(m0)
                .into_iter()
                .filter(|(_, lnk)| {
                    lnk.is_none_or(|(f, t)| {
                        let (bf, bt) = (byte_of(f), byte_of(t));
                        !flagged.iter().any(|r| bf < r.end && r.start < bt)
                    })
                })
                .map(|(p, _)| p)
                .collect();
            if !keep.is_empty() {
                let mut have: HashMap<String, usize> = HashMap::new();
                for (p, _) in content_preds(m1) {
                    *have.entry(p).or_default() += 1;
                }
                let mut kept = 0;
                for p in &keep {
                    if let Some(n) = have.get_mut(p) {
                        if *n > 0 {
                            *n -= 1;
                            kept += 1;
                        }
                    }
                }
                let need = match c.source {
                    Source::Model => self.rewrite.keep_meaning,
                    _ => 1.0,
                };
                let share = kept as f64 / keep.len() as f64;
                if share + 1e-9 < need {
                    return Some(format!(
                        "changes the meaning (keeps {kept} of {} content words)",
                        keep.len()
                    ));
                }
            }
        }
        None
    }

    /// Propose rewrites for every sentence with diagnostics in `a`.
    pub fn propose(&mut self, a: &Analysis, diags: &[Diagnostic]) -> Vec<Proposal> {
        let mut out = Vec::new();
        for (si, s) in a.sentences.iter().enumerate() {
            // Sentences with masked inline code are left alone.
            if s.text != s.original {
                continue;
            }
            let src = a.sentence_source(si);
            let mine: Vec<Diagnostic> = diags
                .iter()
                .filter(|d| d.sentence == Some(si))
                .filter(|d| {
                    self.rewrite.rules.is_empty()
                        || self
                            .rewrite
                            .rules
                            .iter()
                            .any(|r| d.rule.starts_with(r.as_str()))
                })
                .map(|d| {
                    let mut d = d.clone();
                    d.range = d.range.start.saturating_sub(src.start)
                        ..d.range.end.saturating_sub(src.start);
                    d
                })
                .collect();
            if mine.is_empty() {
                continue;
            }
            let sentence = s.original.clone();
            let issues: Vec<String> = mine.iter().map(|d| d.message.clone()).collect();
            let flagged: Vec<std::ops::Range<usize>> =
                mine.iter().map(|d| d.range.clone()).collect();
            // The original's diagnostics as a single sentence, so that the
            // comparison is like for like.
            let (_, base) = self.check_one(&sentence);
            let mut candidates = self.deterministic(&sentence, &mine);
            candidates.extend(self.model_candidates(&sentence, &issues));
            for c in candidates.iter_mut() {
                c.rejected = self.verify(c, s, &base, &flagged);
            }
            if let Some(model) = self.model.as_mut() {
                for c in candidates.iter_mut().filter(|c| c.rejected.is_none()) {
                    c.lm_score = model
                        .logprob("", &c.text)
                        .map(|(lp, n)| lp / n.max(1) as f64);
                }
            }
            // The model's likelihood decides; without a model, or on a
            // tie, the order of the candidates does (the diagnostics' own
            // fixes first, then spelling suggestions in their ranked order,
            // then model rewrites), then the smaller edit.
            let chosen = candidates
                .iter()
                .enumerate()
                .filter(|(_, c)| c.rejected.is_none())
                .max_by(|(i, x), (j, y)| {
                    let lx = x.lm_score.unwrap_or(0.0);
                    let ly = y.lm_score.unwrap_or(0.0);
                    lx.total_cmp(&ly).then(j.cmp(i)).then(
                        edit_distance(&sentence, &y.text).cmp(&edit_distance(&sentence, &x.text)),
                    )
                })
                .map(|(_, c)| c.clone());
            out.push(Proposal {
                sentence: si,
                original: sentence,
                issues,
                chosen,
                candidates,
            });
        }
        out
    }
}

/// Apply accepted proposals to the source text.
pub fn apply(a: &Analysis, proposals: &[Proposal]) -> (String, usize) {
    let mut edits: Vec<(std::ops::Range<usize>, &str)> = proposals
        .iter()
        .filter_map(|p| {
            p.chosen
                .as_ref()
                .map(|c| (a.sentence_source(p.sentence), c.text.as_str()))
        })
        .collect();
    edits.sort_by_key(|(r, _)| r.start);
    let mut out = String::new();
    let mut pos = 0;
    let mut n = 0;
    for (r, text) in edits {
        if r.start < pos {
            continue;
        }
        out.push_str(&a.source[pos..r.start]);
        out.push_str(text);
        pos = r.end;
        n += 1;
    }
    out.push_str(&a.source[pos..]);
    (out, n)
}
