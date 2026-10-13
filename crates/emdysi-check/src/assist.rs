//! Spelling and grammar fixes that combine the grammar with a small local
//! decision model (decision D12): the grammar proposes and verifies, the
//! model chooses in context.
//!
//! - Grammar: a sentence with no strict analysis gets candidate
//!   corrections that change one word to another form of the same word
//!   (|jump| to |jumps|); only those the strict grammar accepts are kept,
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
const MAX_CANDIDATES: usize = 24;
/// Most corrected versions shown to the model with the original.
const MAX_OPTIONS: usize = 5;
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

fn strict(erg: &Erg, text: &str) -> bool {
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

/// Corrections of one word to another form of it that the strict grammar
/// accepts: (from, to, word, replacement, corrected sentence).
fn grammar_candidates(erg: &Erg, s: &Sentence) -> Vec<(usize, usize, String, String, String)> {
    let Some(best) = s.best() else {
        return Vec::new();
    };
    let chars: Vec<char> = s.original.chars().collect();
    let mut tried = 0;
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
                || tried >= MAX_CANDIDATES
            {
                continue;
            }
            tried += 1;
            let rep = same_case(&surface, &f);
            let fixed = replace(&s.original, w.from, w.to, &rep);
            if strict(erg, &fixed) {
                out.push((w.from, w.to, surface.clone(), rep, fixed));
            }
        }
    }
    out
}

/// Grammar corrections chosen by the model (see the module comment).
pub fn grammar(erg: &Erg, a: &Analysis, ask: &mut dyn Ask, threshold: f64) -> Vec<Correction> {
    let mut out = Vec::new();
    for (si, s) in a.sentences.iter().enumerate() {
        let words = s.word_count();
        if s.strict() || s.parse.is_none() || !(3..=30).contains(&words) {
            continue;
        }
        let cands = grammar_candidates(erg, s);
        if cands.is_empty() {
            continue;
        }
        let options: Vec<&str> = std::iter::once(s.original.trim())
            .chain(cands.iter().take(MAX_OPTIONS).map(|c| c.4.trim()))
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
            let (from, to, text, rep, _) = &cands[best - 1];
            out.push(Correction {
                sentence: si,
                from: *from,
                to: *to,
                text: text.clone(),
                replacement: rep.clone(),
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
