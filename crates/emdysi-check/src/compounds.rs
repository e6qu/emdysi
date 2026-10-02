//! Compounds: hyphen chains ("decision-making-framework",
//! "single-source-of-truth"), hyphens after -ly adverbs
//! ("highly-available") and noun stacks ("customer data platform
//! integration strategy"), read from the grammar's analysis rather than
//! from hyphens alone.
//!
//! Style guides agree on the basis: Chicago, AP, Google and Microsoft
//! hyphenate a compound modifier before its noun ("long-term care"), never
//! after an -ly adverb, and none fuses a modifier onto its head noun;
//! plainlanguage.gov says that three nouns in a row hurt readability and
//! "once you get past three, the string becomes unbearable".

use emdysi_parse::Word;

use crate::structure::Hit;
use crate::terms::Glossary;
use crate::{Analysis, Sentence};

/// Hyphenated words and phrases of a sentence: (from, to, text, segment
/// spans), outside inline code.
/// A hyphenated word: character span, text and the spans of its parts.
type Hyphenated = (usize, usize, String, Vec<(usize, usize)>);

fn hyphenated(s: &Sentence) -> Vec<Hyphenated> {
    let chars: Vec<char> = s.original.chars().collect();
    let masked: Vec<char> = s.text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_alphanumeric()
            || (i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '-'))
        {
            i += 1;
            continue;
        }
        let start = i;
        let mut segs = Vec::new();
        let mut seg_start = i;
        while i < chars.len()
            && (chars[i].is_alphanumeric()
                || chars[i] == '\''
                || chars[i] == '’'
                || (chars[i] == '-'
                    && chars.get(i + 1).is_some_and(|c| c.is_alphanumeric())
                    && i > seg_start))
        {
            if chars[i] == '-' {
                segs.push((seg_start, i));
                seg_start = i + 1;
            }
            i += 1;
        }
        segs.push((seg_start, i));
        let code = masked
            .get(start..i)
            .is_some_and(|m| m.iter().all(|&c| c == 'x' || c == '-'))
            && !chars[start..i].iter().all(|&c| c == 'x' || c == '-');
        if segs.len() >= 2 && !code {
            out.push((start, i, chars[start..i].iter().collect(), segs));
        }
    }
    out
}

/// The word of the best reading at a character span (exactly, or the
/// first one starting there).
fn word_at(s: &Sentence, from: usize, to: usize) -> Option<&Word> {
    let r = s.best()?;
    r.words
        .iter()
        .find(|w| w.from == from && w.to == to)
        .or_else(|| r.words.iter().find(|w| w.from == from))
}

fn is_noun(w: &Word) -> bool {
    w.le_type.starts_with("n_")
}

/// The next word after character `to`, if it follows a space.
fn next_word(s: &Sentence, to: usize) -> Option<&Word> {
    let r = s.best()?;
    if s.original.chars().nth(to) != Some(' ') {
        return None;
    }
    r.words
        .iter()
        .filter(|w| w.from > to)
        .min_by_key(|w| w.from)
}

const NUMBERS: &[&str] = &[
    "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten", "twenty",
    "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety", "hundred", "thousand",
    "half", "third", "thirds", "quarter", "quarters", "fifths",
];

/// Hyphen chains of three or more parts that the grammar does not know as
/// one word: a modifier fused onto its head noun gets the last hyphen
/// dropped ("decision-making framework"); a chain used as a noun is better
/// written open ("single source of truth"); a chain of four or more parts
/// before a noun is better rephrased.
pub fn run_kebab(a: &Analysis, except: &[String]) -> Vec<Hit> {
    let mut out = Vec::new();
    for (si, s) in a.sentences.iter().enumerate() {
        let Some(r) = s.best() else { continue };
        for (from, to, text, segs) in hyphenated(s) {
            if segs.len() < 3 || except.contains(&text.to_lowercase()) {
                continue;
            }
            let parts: Vec<String> = segs
                .iter()
                .map(|&(f, t)| {
                    s.original
                        .chars()
                        .skip(f)
                        .take(t - f)
                        .collect::<String>()
                        .to_lowercase()
                })
                .collect();
            if parts
                .iter()
                .all(|p| NUMBERS.contains(&p.as_str()) || p.chars().all(|c| c.is_ascii_digit()))
            {
                continue;
            }
            // Known to the grammar as one word ("easy-to-use").
            if r.words
                .iter()
                .any(|w| w.from <= from && w.to >= to && w.lemma.contains(' '))
            {
                continue;
            }
            let (lf, lt) = *segs.last().unwrap();
            let (pf, pt) = segs[segs.len() - 2];
            let last = word_at(s, lf, lt);
            let prev = word_at(s, pf, pt);
            let before_noun = next_word(s, to).is_some_and(is_noun);
            let fused = !before_noun
                && last.is_some_and(is_noun)
                && prev
                    .is_some_and(|w| w.le_type.starts_with("v_") || w.le_type.starts_with("aj_"));
            let mut h = Hit::at(a, si, from, to);
            if fused {
                let head: String = s.original.chars().skip(lf).take(lt - lf).collect();
                let modifier: String = s.original.chars().skip(from).take(pt - from).collect();
                let rep = format!("{modifier} {head}");
                h = h
                    .var("fix", &rep)
                    .var("kind", "a modifier fused onto its noun");
                h.replacement = Some(rep);
            } else if !before_noun {
                let rep = text.replace('-', " ");
                h = h.var("fix", &rep).var("kind", "a hyphenated noun phrase");
                h.suggestions = vec![rep];
            } else if segs.len() >= 4 {
                h = h
                    .var("fix", "a relative clause or a prepositional phrase")
                    .var("kind", "a long chain of modifiers");
            } else {
                continue;
            }
            out.push(h);
        }
    }
    out
}

/// A hyphen after an -ly adverb ("highly-available"): the adverb already
/// marks what it modifies, so the hyphen goes.
pub fn run_ly_hyphen(a: &Analysis) -> Vec<Hit> {
    let mut out = Vec::new();
    for (si, s) in a.sentences.iter().enumerate() {
        for (from, _, _, segs) in hyphenated(s) {
            let (f, t) = segs[0];
            let first: String = s.original.chars().skip(f).take(t - f).collect();
            if !first.to_lowercase().ends_with("ly") {
                continue;
            }
            let Some(w) = word_at(s, f, t) else { continue };
            if !w.le_type.starts_with("av_") {
                continue;
            }
            let (nf, nt) = segs[1];
            let next: String = s.original.chars().skip(nf).take(nt - nf).collect();
            let mut h = Hit::at(a, si, from, nt);
            let rep = format!("{first} {next}");
            h = h.var("fix", &rep);
            h.replacement = Some(rep);
            out.push(h);
        }
    }
    out
}

/// Noun-noun compounds of `min` to `max` nouns ("customer data platform
/// integration strategy"), from the grammar's compound rules. Proper names
/// and multiword glossary terms count as one noun; words the grammar does
/// not know are not counted.
pub fn run_noun_stacks(a: &Analysis, min: usize, max: usize, g: &Glossary) -> Vec<Hit> {
    let mut out = Vec::new();
    for (si, s) in a.sentences.iter().enumerate() {
        let Some(r) = s.best() else { continue };
        let cpd = |name: &str| name.starts_with("n-hdn_cpd");
        let spans: Vec<(usize, usize)> = r
            .nodes
            .iter()
            .filter(|n| !n.leaf && cpd(&n.name))
            .map(|n| (n.from, n.to))
            .collect();
        let terms = g.term_spans(s);
        for &(from, to) in &spans {
            // Only the largest compound.
            if spans
                .iter()
                .any(|&(f, t)| f <= from && to <= t && (f, t) != (from, to))
            {
                continue;
            }
            let nouns: Vec<&Word> = r
                .words
                .iter()
                .filter(|w| {
                    w.from >= from && w.to <= to && is_noun(w) && !w.le_type.starts_with("n_-_pn")
                })
                .collect();
            let generic = nouns.iter().filter(|w| w.generic).count();
            // A run of capitalized nouns is a name: one unit.
            let first_word = r.words.iter().map(|w| w.from).min().unwrap_or(0);
            let cap = |w: &Word| {
                w.from != first_word && w.surface.chars().next().is_some_and(char::is_uppercase)
            };
            let mut names = 0;
            for k in 1..nouns.len() {
                if cap(nouns[k]) && cap(nouns[k - 1]) {
                    names += 1;
                }
            }
            // Words unknown to the grammar get a default noun analysis
            // ("meticulous"), so they do not count.
            let mut n = nouns.len().saturating_sub(generic).saturating_sub(names);
            for &(tf, tt) in &terms {
                if tf >= from && tt <= to {
                    let inside = nouns.iter().filter(|w| w.from >= tf && w.to <= tt).count();
                    n = n.saturating_sub(inside.saturating_sub(1));
                }
            }
            if n >= min && n <= max {
                // Trim punctuation the span may include.
                let text: String = s.original.chars().skip(from).take(to - from).collect();
                let trimmed = text.trim_end_matches(|c: char| !c.is_alphanumeric());
                let end = from + trimmed.chars().count();
                out.push(Hit::at(a, si, from, end).var("count", n));
            }
        }
    }
    out
}
