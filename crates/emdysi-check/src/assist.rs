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
//!   and the model is asked which version of the sentence is right (in
//!   rounds of nine versions when there are more, then among the rounds'
//!   choices). The model is first asked which word is wrong, and the
//!   edits at that word are tried first. An
//!   error is reported only when the grammar accepts the correction and the
//!   model prefers it to the original with probability `threshold` or
//!   more. Neither alone is enough: the grammar cannot tell a noun-phrase
//!   reading (|the quick brown fox jump|, a jump) from an error, and the
//!   model can prefer a sentence the grammar rejects.
//! - Spelling: when several known words are one edit from an unknown word,
//!   the model chooses among them in the sentence; the choice becomes the
//!   fix when it is likely enough and the sentence with it parses as well
//!   as with any other candidate.

use std::time::{Duration, Instant};

use emdysi_parse::{Erg, InputToken};

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

/// Lengths of the sentences that get candidate corrections, in words. Up
/// to 50, the seven real errors in sentences of 41 to 46 words gained
/// nothing: their corrected sentences do not parse strictly in time.
pub const WORDS: std::ops::RangeInclusive<usize> = 3..=40;
/// Most candidate sentences parsed for one sentence.
pub const MAX_CANDIDATES: usize = 120;
/// Most corrected versions shown to the model with the original in one
/// question (the decision engine takes up to ten options).
pub const MAX_OPTIONS: usize = 9;
/// Most questions of `MAX_OPTIONS` versions each; their choices are then
/// put to the model together.
pub const MAX_ROUNDS: usize = 3;

/// Function words that writers drop, add or confuse with another of their
/// class (from the real-error corpus: |are| for |is|, a missing |the|,
/// |for| for |to|, a doubled |and|, |which| for |that|, |or| for |and|).
/// A word may be in several classes.
const CLASSES: &[&[&str]] = &[
    &["a", "an", "the"],
    &["the", "they", "then"],
    &["is", "are", "was", "were", "be"],
    &["do", "does", "did"],
    &["has", "have", "had"],
    &["can", "could", "should", "will", "would", "may", "must"],
    &[
        "in", "on", "at", "for", "to", "of", "from", "by", "with", "into", "about", "as", "within",
    ],
    &["it", "its", "it's", "they", "them", "their"],
    &["their", "its", "your", "our", "his", "her"],
    &["you", "we", "they"],
    &["this", "these", "that", "those", "the"],
    &["which", "that", "who"],
    &["and", "or"],
    // A dropped or added negation (|does already contain| for |doesn't|).
    &["does", "doesn't"],
    &["do", "don't"],
    &["is", "isn't"],
    &["are", "aren't"],
    &["can", "can't"],
    &["will", "won't"],
];
/// Words tried as insertions before each word.
const INSERTED: &[&str] = &[
    "the", "a", "an", "of", "to", "it", "you", "we", "is", "be", "in", "on", "with", "this",
    "should", "to be",
];
/// Time limit for parsing a spelling candidate, and for a corrected
/// sentence of up to `SHORT` words; longer ones get `PER_WORD` more per
/// word, up to `LONGEST` (the checker's own limit).
const PARSE_TIMEOUT: Duration = Duration::from_secs(2);
const SHORT: usize = 15;
const PER_WORD: Duration = Duration::from_millis(320);
const LONGEST: Duration = Duration::from_secs(10);
/// Total time for verifying the candidate corrections of one sentence: a
/// candidate that does not parse can take its whole limit, and a doubted
/// sentence has up to `MAX_CANDIDATES` of them.
const BUDGET: Duration = Duration::from_secs(30);

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
        capitalized(rep)
    } else {
        rep.to_string()
    }
}

/// `rep` with a capital first letter.
fn capitalized(rep: &str) -> String {
    let mut c = rep.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// Whether the strict grammar accepts `text` (within a short time limit).
pub fn strict(erg: &Erg, text: &str) -> bool {
    strict_within(erg, text, PARSE_TIMEOUT)
}

fn strict_within(erg: &Erg, text: &str, limit: Duration) -> bool {
    erg.parse_limited(text.trim(), limit, 3)
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

/// A function-word edit: (from, to, replaced text, replacement).
type WordEdit = (usize, usize, String, String);

/// Edits of a function word: another of its class or deletion, then,
/// separately, one of `INSERTED` before a word.
fn function_word_edits(s: &Sentence) -> (Vec<WordEdit>, Vec<WordEdit>) {
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
        let classes: Vec<_> = CLASSES
            .iter()
            .filter(|c| c.contains(&lower.as_str()))
            .collect();
        if !classes.is_empty() {
            let mut seen = Vec::new();
            for other in classes.iter().flat_map(|c| c.iter()) {
                if *other != lower && !seen.contains(other) {
                    seen.push(*other);
                    // A sentence starts with a capital, whatever the
                    // word it starts with now.
                    let rep = if t.from == 0 {
                        capitalized(other)
                    } else {
                        same_case(&surface, other)
                    };
                    subs.push((t.from, t.to, surface.clone(), rep));
                }
            }
            // A modal before |be| (|The output is similar| for |should
            // be similar|).
            if matches!(lower.as_str(), "is" | "are") {
                for m in ["should", "can", "will", "must"] {
                    let rep = format!("{m} be");
                    let rep = if t.from == 0 { capitalized(&rep) } else { rep };
                    subs.push((t.from, t.to, surface.clone(), rep));
                }
            }
            // The word and the space after it (or before it, at the end);
            // with its comma, a word set off by one (|as of this writing,
            // this, it compiles|).
            if chars.get(t.to) == Some(&',') && chars.get(t.to + 1) == Some(&' ') {
                dels.push((t.from, t.to + 2, format!("{surface}, "), String::new()));
            } else if chars.get(t.to) == Some(&' ') {
                dels.push((t.from, t.to + 1, format!("{surface} "), String::new()));
            } else if t.from > 0 && chars[t.from - 1] == ' ' {
                dels.push((t.from - 1, t.to, format!(" {surface}"), String::new()));
            }
        }
        if surface.chars().next().is_some_and(char::is_alphabetic) {
            for w in INSERTED {
                let w = if t.from == 0 {
                    capitalized(w)
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
    // Two function words together (|the example of the flow| for |the
    // example flow|).
    let function = |t: &InputToken| {
        t.to <= chars.len() && {
            let w: String = chars[t.from..t.to]
                .iter()
                .collect::<String>()
                .to_lowercase();
            CLASSES.iter().any(|c| c.contains(&w.as_str()))
        }
    };
    for w in s.tokens.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if function(a)
            && function(b)
            && chars.get(a.to) == Some(&' ')
            && a.to + 1 == b.from
            && chars.get(b.to) == Some(&' ')
        {
            let text: String = chars[a.from..=b.to].iter().collect();
            dels.push((a.from, b.to + 1, text, String::new()));
        }
    }
    // A pronoun after the last word, the object of a preposition left
    // without one (|the files against| for |the files against it|).
    // (The last word, before any final punctuation.)
    let all = |t: &InputToken, f: fn(&char) -> bool| {
        t.to <= chars.len() && chars[t.from..t.to].iter().all(f)
    };
    let last = s
        .tokens
        .iter()
        .rposition(|t| all(t, |c| c.is_alphabetic()))
        .filter(|&i| {
            s.tokens[i + 1..]
                .iter()
                .all(|t| all(t, |c| c.is_ascii_punctuation()))
        });
    if let Some(t) = last.map(|i| &s.tokens[i]) {
        let surface: String = chars[t.from..t.to].iter().collect();
        for w in ["it", "them"] {
            ins.push((t.from, t.to, surface.clone(), format!("{surface} {w}")));
        }
    }
    let n = subs.len() + dels.len();
    let mut all: Vec<_> = subs.into_iter().chain(dels).chain(ins).collect();
    let ins = all.split_off(n);
    (all, ins)
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
/// a doubled word dropped, one word changed to another listed form of
/// itself, a function word changed or dropped, a real-word typo
/// corrected, a function word inserted; for a sentence without a strict
/// reading,
/// alternating with the edits nearest where its analysis breaks down.
pub fn edits(erg: &Erg, s: &Sentence) -> Vec<Edit> {
    let chars: Vec<char> = s.original.chars().collect();
    let mut out = Vec::new();
    // The words of the best reading, if any: a sentence without even a
    // fragment reading (|Binding objects link the policy to your cluster
    // and provides scoping.|) still gets the edits of its tokens below.
    for w in s.best().iter().flat_map(|b| b.words.iter()) {
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
        // The other forms of the word and of its lemma (`inflections`
        // leaves out the word it is given), a noun's only as nouns.
        let lower = surface.to_lowercase();
        let mut forms = erg.inflections(&lower);
        forms.push(w.lemma.to_lowercase());
        forms.extend(erg.inflections(&w.lemma));
        if w.le_type.starts_with("n_") {
            forms.retain(|f| (!f.ends_with("ed") && !f.ends_with("ing")) || lower.ends_with("ing"));
        }
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
    // The other listed forms of every plain word, as far as the reading did
    // not give them: a word the best reading did not take as a noun or verb
    // (in a fragment reading, |provides| may be read as anything), or took
    // as one where it is the other (|checks| read as a noun, for
    // |checked|).
    for t in &s.tokens {
        if t.to > chars.len() {
            continue;
        }
        let surface: String = chars[t.from..t.to].iter().collect();
        if surface.chars().count() < REAL_WORD_MIN
            || !surface.chars().all(|c| c.is_ascii_alphabetic())
            || surface.chars().skip(1).any(|c| c.is_ascii_uppercase())
        {
            continue;
        }
        let mut forms = erg.inflections(&surface.to_lowercase());
        forms.sort();
        forms.dedup();
        for f in forms {
            let rep = same_case(&surface, &f);
            if f.eq_ignore_ascii_case(&surface)
                || f.contains(' ')
                || crate::dict::tier(&f.to_lowercase()).is_none()
                || out
                    .iter()
                    .any(|e: &Edit| e.from == t.from && e.replacement == rep)
            {
                continue;
            }
            out.push(Edit {
                from: t.from,
                to: t.to,
                text: surface.clone(),
                replacement: rep,
            });
        }
    }
    let as_edit = |(from, to, text, replacement): WordEdit| Edit {
        from,
        to,
        text,
        replacement,
    };
    // A sentence adverb written as its adjective (|Additional, we are
    // starting ...|): the first word, before a comma, with |-ly|.
    if let Some(t) = s.tokens.first().filter(|t| chars.get(t.to) == Some(&',')) {
        let surface: String = chars[t.from..t.to].iter().collect();
        let adverb = format!("{}ly", surface.to_lowercase());
        if surface.chars().all(char::is_alphabetic) && crate::dict::tier(&adverb).is_some() {
            out.push(Edit {
                from: t.from,
                to: t.to,
                text: surface.clone(),
                replacement: same_case(&surface, &adverb),
            });
        }
    }
    let (changed, inserted) = function_word_edits(s);
    out.extend(changed.into_iter().map(as_edit));
    // A relative pronoun and the verb after it, changed together (|which
    // has status| for |that have status|).
    for w in s.tokens.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if b.to > chars.len() || chars.get(a.to) != Some(&' ') || a.to + 1 != b.from {
            continue;
        }
        let pronoun: String = chars[a.from..a.to].iter().collect();
        let verb: String = chars[b.from..b.to].iter().collect();
        let lower = pronoun.to_lowercase();
        if !matches!(lower.as_str(), "which" | "that" | "who")
            || !verb.chars().all(|c| c.is_ascii_lowercase())
        {
            continue;
        }
        for other in ["which", "that", "who"].into_iter().filter(|o| *o != lower) {
            for f in erg.inflections(&verb) {
                if f == verb || f.contains(' ') || crate::dict::tier(&f).is_none() {
                    continue;
                }
                out.push(Edit {
                    from: a.from,
                    to: b.to,
                    text: format!("{pronoun} {verb}"),
                    replacement: format!("{} {f}", same_case(&pronoun, other)),
                });
            }
        }
    }
    let mut real = real_word_edits(s);
    real.retain(|e| !out.contains(e));
    out.extend(real);
    // Related words, before the insertions: another word on the same
    // stem a few letters away (|iteration| for |iterator|, |extendable|
    // for |extensible|, |worse| for |worst|).
    for t in &s.tokens {
        if t.to > chars.len() {
            continue;
        }
        let surface: String = chars[t.from..t.to].iter().collect();
        let lower = surface.to_lowercase();
        let n = lower.chars().count();
        if n < 5
            || !lower.chars().all(|c| c.is_ascii_lowercase())
            || surface.chars().skip(1).any(|c| c.is_ascii_uppercase())
        {
            continue;
        }
        let shared = n.min(6) - 1;
        for x in crate::dict::by_prefix(&lower[..4]) {
            let rep = same_case(&surface, x);
            if *x == lower
                || x.len().abs_diff(n) > 4
                || lower
                    .chars()
                    .zip(x.chars())
                    .take_while(|(a, b)| a == b)
                    .count()
                    < shared
                || crate::rules::edit_distance(&lower, x) > 3
                || out.iter().any(|e| e.from == t.from && e.replacement == rep)
            {
                continue;
            }
            out.push(Edit {
                from: t.from,
                to: t.to,
                text: surface.clone(),
                replacement: rep,
            });
        }
    }
    out.extend(inserted.into_iter().map(as_edit));
    // A doubled word (|the the|) dropped first.
    let mut doubled = Vec::new();
    for w in s.tokens.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if b.to > chars.len() || chars.get(a.to) != Some(&' ') || a.to + 1 != b.from {
            continue;
        }
        let (x, y): (String, String) = (
            chars[a.from..a.to].iter().collect(),
            chars[b.from..b.to].iter().collect(),
        );
        if x.eq_ignore_ascii_case(&y) && x.chars().all(char::is_alphabetic) {
            doubled.push(Edit {
                from: a.to,
                to: b.to,
                text: format!(" {y}"),
                replacement: String::new(),
            });
        }
    }
    out.retain(|e| {
        !doubled
            .iter()
            .any(|d| d.apply(&s.original) == e.apply(&s.original))
    });
    let out: Vec<Edit> = doubled.into_iter().chain(out).collect();
    // For a fragment reading, the edits in this order alternate with the
    // same edits by nearness to its seams (the error is often where the
    // partial analyses meet, but not always).
    let seams = seams(s);
    if !seams.is_empty() {
        let distance = |e: &Edit| {
            seams
                .iter()
                .map(|&p| {
                    if e.from <= p && p <= e.to {
                        0
                    } else {
                        p.abs_diff(e.from).min(p.abs_diff(e.to))
                    }
                })
                .min()
                .unwrap_or(0)
        };
        let mut near = out.clone();
        near.sort_by_key(distance);
        let mut merged: Vec<Edit> = Vec::with_capacity(out.len());
        for (a, b) in out.into_iter().zip(near) {
            for e in [a, b] {
                if !merged.contains(&e) {
                    merged.push(e);
                }
            }
        }
        return merged;
    }
    out
}

/// Where the partial analyses of a sentence without a strict reading
/// meet: the boundaries between the children of the top of its best
/// reading (below any single-child nodes), as character positions.
fn seams(s: &Sentence) -> Vec<usize> {
    if s.strict() {
        return Vec::new();
    }
    let Some(r) = s.best() else {
        return Vec::new();
    };
    let mut i = 0;
    while r.nodes[i].children.len() == 1 && !r.nodes[i].leaf {
        i = r.nodes[i].children[0];
    }
    let kids = &r.nodes[i].children;
    kids.windows(2).map(|w| r.nodes[w[1]].from).collect()
}

/// Whether the strict grammar accepts `s` with the edit `e`, parsed as the
/// checker parses it (inline code masked) and given time by its length:
/// a long sentence that took the checker seconds to parse would otherwise
/// time out corrected.
pub fn verify(erg: &Erg, s: &Sentence, e: &Edit) -> bool {
    verify_within(erg, s, e, LONGEST)
}

/// [`verify`] within at most `most`.
fn verify_within(erg: &Erg, s: &Sentence, e: &Edit, most: Duration) -> bool {
    let start = Instant::now();
    let masked = s.text.chars().count() == s.original.chars().count();
    let text = if masked { &s.text } else { &s.original };
    let extra = s.word_count().saturating_sub(SHORT) as u32;
    let limit = (PARSE_TIMEOUT + PER_WORD * extra).min(LONGEST).min(most);
    strict_within(erg, &with_full_stop(&e.apply(text)), limit)
        || s.word_count() >= LOCAL_WORDS
            && verify_locally(erg, text, e, most.saturating_sub(start.elapsed()))
}

/// Shortest sentence whose corrections may be verified clause by clause.
const LOCAL_WORDS: usize = 12;

/// Whether `e` turns the part of `text` around it, between punctuation
/// marks, from no strict reading into one: a long sentence whose clauses
/// each parse can fail to parse whole within its time (search, M8), and
/// its correction then never verifies. The window grows by one mark on
/// each side, up to three times, and never reaches the whole sentence.
fn verify_locally(erg: &Erg, text: &str, e: &Edit, most: Duration) -> bool {
    let start = Instant::now();
    let chars: Vec<char> = text.chars().collect();
    let marks: Vec<usize> = chars
        .iter()
        .enumerate()
        .filter(|(_, c)| matches!(c, ',' | ';' | ':' | '(' | ')' | '—' | '–'))
        .map(|(i, _)| i)
        .collect();
    let before: Vec<usize> = marks
        .iter()
        .rev()
        .filter(|&&m| m < e.from)
        .copied()
        .collect();
    let after: Vec<usize> = marks.iter().filter(|&&m| m >= e.to).copied().collect();
    for k in 0..3 {
        // The window: after the k-th mark before the edit, up to the k-th
        // mark after it.
        let from = before.get(k).map_or(0, |m| m + 1);
        let to = after.get(k).copied().unwrap_or(chars.len());
        if from == 0 && to == chars.len() {
            break;
        }
        let part: String = chars[from..to].iter().collect();
        let fixed = Edit {
            from: e.from - from,
            to: e.to - from,
            text: e.text.clone(),
            replacement: e.replacement.clone(),
        }
        .apply(&part);
        let words = part.split_whitespace().count();
        let limit = (PARSE_TIMEOUT + PER_WORD * words.saturating_sub(SHORT) as u32)
            .min(LONGEST)
            .min(most.saturating_sub(start.elapsed()));
        if limit.is_zero() {
            return false;
        }
        if strict_within(erg, &with_full_stop(&fixed), limit) {
            return !strict_within(erg, &with_full_stop(&part), limit);
        }
        if before.get(k).is_none() && after.get(k).is_none() {
            break;
        }
    }
    false
}

/// `text` ending in a full stop: one in place of a final colon, or added
/// when it has no final punctuation. The strict grammar wants sentence-final
/// punctuation, and a list item or a sentence cut short by a code block
/// (|On each node, restart the kubelet|) would otherwise never verify,
/// whatever its correction.
fn with_full_stop(text: &str) -> String {
    let t = text.trim_end();
    if let Some(head) = t.strip_suffix(':') {
        format!("{head}.")
    } else if t.ends_with(['.', '!', '?']) || t.ends_with(['"', '”', ')']) {
        t.to_string()
    } else {
        format!("{t}.")
    }
}

/// The first `MAX_OPTIONS * MAX_ROUNDS` of the first `MAX_CANDIDATES`
/// edits of `s` that the strict grammar accepts, with the corrected
/// sentence. Parsing stops once enough are found (the model is not shown
/// more) or when `BUDGET` is spent.
fn grammar_candidates(erg: &Erg, s: &Sentence, focus: &[(usize, usize)]) -> Vec<(Edit, String)> {
    let start = Instant::now();
    focused(edits(erg, s), focus)
        .into_iter()
        .take(MAX_CANDIDATES)
        .take_while(|_| start.elapsed() + Duration::from_millis(200) < BUDGET)
        .filter(|e| verify_within(erg, s, e, BUDGET.saturating_sub(start.elapsed())))
        .map(|e| {
            let fixed = e.apply(&s.original);
            (e, fixed)
        })
        .take(MAX_OPTIONS * MAX_ROUNDS)
        .collect()
}

/// The edits that touch one of the `focus` spans first, the others after
/// them, each in their order.
pub fn focused(edits: Vec<Edit>, focus: &[(usize, usize)]) -> Vec<Edit> {
    let (mut near, far): (Vec<Edit>, Vec<Edit>) = edits
        .into_iter()
        .partition(|e| focus.iter().any(|&(f, t)| e.from < t && f < e.to));
    near.extend(far);
    near
}

/// The question that asks the model where a doubted sentence goes wrong;
/// the sentence follows it, and the options are its words
/// ([`word_options`]).
pub const LOCATE_QUESTION: &str =
    "Which word of the sentence is wrong? (If a word is missing, the word just after the gap.)";

/// The words of `s` the model can point at, as (from, to, label): tokens
/// with a letter or digit, a repeated word labelled with its occurrence
/// (|the|, |the (2)|).
pub fn word_options(s: &Sentence) -> Vec<(usize, usize, String)> {
    let chars: Vec<char> = s.original.chars().collect();
    let mut out: Vec<(usize, usize, String)> = Vec::new();
    for t in &s.tokens {
        if t.to > chars.len() {
            continue;
        }
        let surface: String = chars[t.from..t.to].iter().collect();
        if !surface.chars().any(char::is_alphanumeric) {
            continue;
        }
        let seen = out
            .iter()
            .filter(|o| o.2 == surface || o.2.starts_with(&format!("{surface} (")))
            .count();
        let label = if seen == 0 {
            surface
        } else {
            format!("{surface} ({})", seen + 1)
        };
        out.push((t.from, t.to, label));
    }
    out
}

/// The word the model says is wrong, as its span (none if it cannot
/// answer): the words in groups of ten, then the groups' choices.
fn locate(ask: &mut dyn Ask, context: &str, s: &Sentence) -> Vec<(usize, usize)> {
    let words = word_options(s);
    let question = format!("{LOCATE_QUESTION}\n\nSentence: {}", s.original.trim());
    let mut best = |group: &[usize]| -> Option<usize> {
        let labels: Vec<&str> = group.iter().map(|&i| words[i].2.as_str()).collect();
        let probs = ask.ask(context, &question, &labels)?;
        let (i, &p) = probs.iter().enumerate().max_by(|x, y| x.1.total_cmp(y.1))?;
        // An answer that ties at the top names no word: the edits keep
        // their own order rather than favour whichever word came last.
        if probs.iter().filter(|&&q| q == p).count() > 1 {
            return None;
        }
        Some(group[i])
    };
    let all: Vec<usize> = (0..words.len()).collect();
    let chosen = if all.len() <= 10 {
        best(&all)
    } else {
        let winners: Vec<usize> = all.chunks(10).filter_map(&mut best).collect();
        if winners.len() > 1 {
            best(&winners)
        } else {
            winners.first().copied()
        }
    };
    chosen
        .map(|i| vec![(words[i].0, words[i].1)])
        .unwrap_or_default()
}

/// Shortest word whose real-word neighbours are tried (shorter words have
/// many, and the function words among them have their own edits).
const REAL_WORD_MIN: usize = 4;

/// Real-word typos: each word of `REAL_WORD_MIN` letters or more replaced
/// by the common words (the commonest tier of the word list) one edit
/// away from it (|Manger| for |Manager|, |Fore| for |For|, |though| for
/// |thought|): a letter dropped, added, changed, or two swapped.
fn real_word_edits(s: &Sentence) -> Vec<Edit> {
    let chars: Vec<char> = s.original.chars().collect();
    let common = |w: &str| crate::dict::words().get(w).is_some_and(|&t| t <= 35);
    let mut out = Vec::new();
    for t in &s.tokens {
        if t.to > chars.len() {
            continue;
        }
        let surface: String = chars[t.from..t.to].iter().collect();
        // Plain words only, not acronyms or code.
        if surface.chars().count() < REAL_WORD_MIN
            || !surface.chars().all(|c| c.is_ascii_alphabetic())
            || surface.chars().skip(1).any(|c| c.is_ascii_uppercase())
        {
            continue;
        }
        let w = surface.to_lowercase();
        let b: Vec<char> = w.chars().collect();
        let mut near: Vec<String> = Vec::new();
        for i in 0..=b.len() {
            let (head, tail) = (&b[..i], &b[i..]);
            let mut add = |v: Vec<char>| {
                let v: String = v.into_iter().collect();
                if v != w && !near.contains(&v) && common(&v) {
                    near.push(v);
                }
            };
            if !tail.is_empty() {
                add([head, &tail[1..]].concat());
            }
            if tail.len() >= 2 {
                add([head, &[tail[1], tail[0]], &tail[2..]].concat());
            }
            for c in 'a'..='z' {
                if !tail.is_empty() && tail[0] != c {
                    add([head, &[c], &tail[1..]].concat());
                }
                add([head, &[c], tail].concat());
            }
        }
        near.sort();
        for v in near {
            out.push(Edit {
                from: t.from,
                to: t.to,
                text: surface.clone(),
                replacement: same_case(&surface, &v),
            });
        }
    }
    out
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
        if s.parse.is_none() || !WORDS.contains(&words) {
            continue;
        }
        // A sentence the grammar accepts once it ends in a full stop (a
        // list item, |Restart the kubelet|) is grammatical too.
        let grammatical = s.strict() || {
            let text = if s.text.chars().count() == s.original.chars().count() {
                &s.text
            } else {
                &s.original
            };
            let stopped = with_full_stop(text);
            stopped.trim() != text.trim() && strict(erg, &stopped)
        };
        if grammatical {
            let Some(min) = screen else { continue };
            let question = format!("{SCREEN_QUESTION}\n\nSentence: {}", s.original.trim());
            let doubt = ask
                .ask(&context(a, si), &question, &["yes", "no"])
                .map_or(0.0, |p| p[1]);
            if doubt < min {
                continue;
            }
        }
        let ctx = context(a, si);
        // The edits at the word the model says is wrong come first: in a
        // sentence the grammar accepts, most edits parse too.
        let focus = locate(ask, &ctx, s);
        let cands = grammar_candidates(erg, s, &focus);
        if cands.is_empty() {
            continue;
        }
        // The model's choice among the original and `group`, as (index in
        // `group`, probability), when it prefers a correction.
        let mut choose = |group: &[usize]| -> Option<(usize, f64, f64)> {
            let options: Vec<&str> = std::iter::once(s.original.trim())
                .chain(group.iter().map(|&i| cands[i].1.trim()))
                .collect();
            let probs = ask.ask(&ctx, GRAMMAR_QUESTION, &options)?;
            let (best, &p) = probs
                .iter()
                .enumerate()
                .skip(1)
                .max_by(|x, y| x.1.total_cmp(y.1))?;
            Some((group[best - 1], p, probs[0]))
        };
        let all: Vec<usize> = (0..cands.len()).collect();
        let chosen = if cands.len() <= MAX_OPTIONS {
            choose(&all)
        } else {
            // Rounds, then the rounds' choices together.
            let winners: Vec<usize> = all
                .chunks(MAX_OPTIONS)
                .filter_map(|g| choose(g).filter(|&(_, p, orig)| p > orig).map(|c| c.0))
                .collect();
            if winners.is_empty() {
                None
            } else {
                choose(&winners)
            }
        };
        let Some((best, p, orig)) = chosen else {
            continue;
        };
        if p >= threshold && p > orig {
            let e = &cands[best].0;
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
    // The model's choice must parse if another suggestion does; the word
    // as written, when offered, does not count: an unknown word parses as
    // a generic one (|isn't strictly neccesary|).
    let parses: Vec<bool> = cands
        .iter()
        .map(|c| c.0 != word && strict(erg, &c.1))
        .collect();
    if cands[best].0 != word && parses.iter().any(|&b| b) && !parses[best] {
        return None;
    }
    Some((cands[best].0.clone(), p))
}
