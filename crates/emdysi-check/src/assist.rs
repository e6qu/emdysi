//! Spelling and grammar fixes that combine the grammar with a small local
//! decision model (decision D12): the grammar proposes and verifies, the
//! model chooses in context.
//!
//! - Grammar: a sentence with no strict analysis gets candidate
//!   corrections that change one word to another form of the same word
//!   (|jump| to |jumps|), or a function word to another of its class
//!   (|are| to |is|), drop one, or insert one (|the|, |of|); only those
//!   the strict grammar accepts are kept,
//!   (a sentence the grammar accepts is considered only when the model,
//!   asked first, doubts it),
//!   and the model is asked which version of the sentence is right. An
//!   error is reported only when the grammar accepts the correction and the
//!   model prefers it to the original with probability `threshold` or
//!   more. Neither alone is enough: the grammar cannot tell a noun-phrase
//!   reading (|the quick brown fox jump|, a jump) from an error, and the
//!   model can prefer a sentence the grammar rejects.
//! - Spelling: when several known words are one edit from an unknown word,
//!   the model chooses among them in the sentence; the choice becomes the
//!   fix when it is likely enough and the sentence with it parses as well
//!   as with any other candidate.

use std::time::Duration;

use emdysi_parse::Erg;

use crate::decisions::Ask;
use crate::{Analysis, Sentence};

/// A correction found with the model: sentence, character span in it,
/// replacement, and the model's probability for the corrected sentence.
#[derive(Debug, Clone)]
pub struct Correction {
    pub sentence: usize,
    pub from: usize,
    pub to: usize,
    pub text: String,
    pub replacement: String,
    pub probability: f64,
}

/// Most candidate sentences parsed for one sentence.
pub const MAX_CANDIDATES: usize = 80;
/// Most corrected versions shown to the model with the original (the
/// decision engine takes up to ten options).
pub const MAX_OPTIONS: usize = 9;

/// Function words that writers drop, add or confuse with another of their
/// class (from the real-error corpus: |are| for |is|, a missing |the|,
/// |for| for |to|, a doubled |and|).
const CLASSES: &[&[&str]] = &[
    &["a", "an", "the"],
    &["is", "are", "was", "were", "be"],
    &["do", "does", "did"],
    &["has", "have", "had"],
    &[
        "in", "on", "at", "for", "to", "of", "from", "by", "with", "into", "about",
    ],
    &["it", "its", "it's"],
    &["this", "these", "that", "those"],
];
/// Words tried as insertions before each word.
const INSERTED: &[&str] = &["the", "a", "of", "to", "it"];
const PARSE_TIMEOUT: Duration = Duration::from_secs(2);

const GRAMMAR_QUESTION: &str =
    "Which version of the sentence is correct English, as the writer meant it?";
const SPELLING_QUESTION: &str = "Which version of the sentence did the writer mean?";

/// `text` with the characters `from..to` replaced.
fn replace(text: &str, from: usize, to: usize, rep: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    chars[..from]
        .iter()
        .chain(rep.chars().collect::<Vec<_>>().iter())
        .chain(chars[to..].iter())
        .collect()
}

/// The replacement written with the capitalisation of `like`.
fn same_case(like: &str, rep: &str) -> String {
    if like.chars().next().is_some_and(char::is_uppercase) {
        let mut c = rep.chars();
        c.next()
            .map(|f| f.to_uppercase().chain(c).collect())
            .unwrap_or_default()
    } else {
        rep.to_string()
    }
}

/// Whether the strict grammar accepts `text` (within a short time limit).
pub fn strict(erg: &Erg, text: &str) -> bool {
    erg.parse_limited(text.trim(), PARSE_TIMEOUT, 3)
        .is_ok_and(|p| p.readings.iter().any(|r| emdysi_parse::is_strict(&r.root)))
}

/// The sentence before `si` in the same block, as context for the model.
fn context(a: &Analysis, si: usize) -> String {
    si.checked_sub(1)
        .map(|p| &a.sentences[p])
        .filter(|p| p.block == a.sentences[si].block)
        .map(|p| p.original.trim().to_string())
        .unwrap_or_default()
}

/// Edits of a function word: another of its class, deletion, or one of
/// `INSERTED` before a word; as (from, to, replaced text, replacement),
/// in that order of preference.
fn function_word_edits(s: &Sentence) -> Vec<(usize, usize, String, String)> {
    let chars: Vec<char> = s.original.chars().collect();
    let mut subs = Vec::new();
    let mut dels = Vec::new();
    let mut ins = Vec::new();
    for t in &s.tokens {
        if t.to > chars.len() {
            continue;
        }
        let surface: String = chars[t.from..t.to].iter().collect();
        let lower = surface.to_lowercase().replace('’', "'");
        if let Some(class) = CLASSES.iter().find(|c| c.contains(&lower.as_str())) {
            for other in class.iter().filter(|o| **o != lower) {
                subs.push((t.from, t.to, surface.clone(), same_case(&surface, other)));
            }
            // The word and the space after it (or before it, at the end).
            if chars.get(t.to) == Some(&' ') {
                dels.push((t.from, t.to + 1, format!("{surface} "), String::new()));
            } else if t.from > 0 && chars[t.from - 1] == ' ' {
                dels.push((t.from - 1, t.to, format!(" {surface}"), String::new()));
            }
        }
        if surface.chars().next().is_some_and(char::is_alphabetic) {
            for w in INSERTED {
                let w = if t.from == 0 {
                    same_case(&surface, w)
                } else {
                    w.to_string()
                };
                let rest = if t.from == 0 {
                    let mut c = surface.chars();
                    c.next()
                        .map(|f| f.to_lowercase().chain(c).collect::<String>())
                        .unwrap_or_default()
                } else {
                    surface.clone()
                };
                ins.push((t.from, t.to, surface.clone(), format!("{w} {rest}")));
            }
        }
    }
    subs.into_iter().chain(dels).chain(ins).collect()
}

/// A candidate correction of a sentence: the characters `from..to`
/// (`text`) replaced by `replacement`.
#[derive(Debug, Clone, PartialEq)]
pub struct Edit {
    pub from: usize,
    pub to: usize,
    pub text: String,
    pub replacement: String,
}

impl Edit {
    /// The sentence with this edit applied.
    pub fn apply(&self, sentence: &str) -> String {
        replace(sentence, self.from, self.to, &self.replacement)
    }
}

/// Every candidate correction of `s`, unverified, in order of preference:
/// one word changed to another listed form of itself, then the
/// function-word edits.
pub fn edits(erg: &Erg, s: &Sentence) -> Vec<Edit> {
    let Some(best) = s.best() else {
        return Vec::new();
    };
    let chars: Vec<char> = s.original.chars().collect();
    let mut out = Vec::new();
    for w in &best.words {
        if w.generic || !(w.le_type.starts_with("v_") || w.le_type.starts_with("n_")) {
            continue;
        }
        if w.le_type.starts_with("n_-_pn") || w.to > chars.len() {
            continue;
        }
        let surface: String = chars[w.from..w.to].iter().collect();
        if !surface.chars().all(char::is_alphabetic) {
            continue;
        }
        let mut forms = erg.inflections(&w.lemma);
        forms.sort();
        forms.dedup();
        for f in forms {
            // A listed word, not a regular form the morphology allows
            // (|foxs|).
            if f.eq_ignore_ascii_case(&surface)
                || f.contains(' ')
                || crate::dict::tier(&f.to_lowercase()).is_none()
            {
                continue;
            }
            out.push(Edit {
                from: w.from,
                to: w.to,
                text: surface.clone(),
                replacement: same_case(&surface, &f),
            });
        }
    }
    out.extend(
        function_word_edits(s)
            .into_iter()
            .map(|(from, to, text, replacement)| Edit {
                from,
                to,
                text,
                replacement,
            }),
    );
    out
}

/// The first `MAX_OPTIONS` of the first `MAX_CANDIDATES` edits of `s`
/// that the strict grammar accepts, with the corrected sentence. Parsing
/// stops once enough are found: the model is not shown more.
fn grammar_candidates(erg: &Erg, s: &Sentence) -> Vec<(Edit, String)> {
    edits(erg, s)
        .into_iter()
        .take(MAX_CANDIDATES)
        .map(|e| {
            let fixed = e.apply(&s.original);
            (e, fixed)
        })
        .filter(|(_, fixed)| strict(erg, fixed))
        .take(MAX_OPTIONS)
        .collect()
}

/// The question that screens a sentence the grammar accepts; the sentence
/// follows it.
pub const SCREEN_QUESTION: &str = "Is this sentence correct English, as written?";

/// Grammar corrections chosen by the model (see the module comment). A
/// sentence the strict grammar accepts is considered only with `screen`:
/// when the model doubts it (answers "no" with that probability or more),
/// since an error can leave a sentence grammatical (|for| for |to|).
pub fn grammar(
    erg: &Erg,
    a: &Analysis,
    ask: &mut dyn Ask,
    threshold: f64,
    screen: Option<f64>,
) -> Vec<Correction> {
    let mut out = Vec::new();
    for (si, s) in a.sentences.iter().enumerate() {
        let words = s.word_count();
        if s.parse.is_none() || !(3..=40).contains(&words) {
            continue;
        }
        if s.strict() {
            let Some(min) = screen else { continue };
            let question = format!("{SCREEN_QUESTION}\n\nSentence: {}", s.original.trim());
            let doubt = ask
                .ask(&context(a, si), &question, &["yes", "no"])
                .map_or(0.0, |p| p[1]);
            if doubt < min {
                continue;
            }
        }
        let cands = grammar_candidates(erg, s);
        if cands.is_empty() {
            continue;
        }
        let options: Vec<&str> = std::iter::once(s.original.trim())
            .chain(cands.iter().map(|c| c.1.trim()))
            .collect();
        let Some(probs) = ask.ask(&context(a, si), GRAMMAR_QUESTION, &options) else {
            continue;
        };
        let Some((best, &p)) = probs
            .iter()
            .enumerate()
            .skip(1)
            .max_by(|x, y| x.1.total_cmp(y.1))
        else {
            continue;
        };
        if p >= threshold && p > probs[0] {
            let e = &cands[best - 1].0;
            out.push(Correction {
                sentence: si,
                from: e.from,
                to: e.to,
                text: e.text.clone(),
                replacement: e.replacement.clone(),
                probability: p,
            });
        }
    }
    out
}

/// The model's choice among spelling suggestions for the word `at`
/// (sentence, start and end character): the suggestion and its
/// probability, when it is likely enough and the sentence with it parses
/// at least as well as with the others (strictly, if any does).
pub fn spelling(
    erg: &Erg,
    a: &Analysis,
    at: (usize, usize, usize),
    suggestions: &[String],
    ask: &mut dyn Ask,
    threshold: f64,
) -> Option<(String, f64)> {
    let (si, from, to) = at;
    let s = &a.sentences[si];
    let word: String = s.original.chars().skip(from).take(to - from).collect();
    let cands: Vec<(String, String)> = suggestions
        .iter()
        .take(MAX_OPTIONS)
        .map(|x| {
            let rep = same_case(&word, x);
            (rep.clone(), replace(&s.original, from, to, &rep))
        })
        .collect();
    if cands.len() < 2 {
        return None;
    }
    let options: Vec<&str> = cands.iter().map(|c| c.1.trim()).collect();
    let probs = ask.ask(&context(a, si), SPELLING_QUESTION, &options)?;
    let (best, &p) = probs.iter().enumerate().max_by(|x, y| x.1.total_cmp(y.1))?;
    if p < threshold {
        return None;
    }
    let parses: Vec<bool> = cands.iter().map(|c| strict(erg, &c.1)).collect();
    if parses.iter().any(|&b| b) && !parses[best] {
        return None;
    }
    Some((cands[best].0.clone(), p))
}
