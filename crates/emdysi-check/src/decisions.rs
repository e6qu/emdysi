//! Checks answered by a decision model (decision D12).
//!
//! A decision model answers a question with fixed options with a
//! probability for each option (see the `emdysi-lm` crate). The checker
//! does not depend on any model: it asks through [`Ask`], and the rules
//! and steps here do nothing without one.
//!
//! - [`Decide`] rules (`kind = "decide"`) ask a question about each
//!   sentence or block and report it when the flagged answer is likely
//!   enough.
//! - [`disambiguate`] settles close calls between the two best readings of
//!   a sentence by showing the model the sentence bracketed both ways, with
//!   the sentences around it.
//! - [`prefer_document_phrases`] settles them without a model, from the
//!   phrases the rest of the document uses.

use std::collections::HashSet;

use emdysi_parse::Reading;
use emdysi_text::blocks::BlockKind;

use crate::Analysis;
use crate::structure::Hit;

/// Answers a question with fixed options: the probability of each option,
/// or `None` if no answer could be had.
pub trait Ask {
    fn ask(&mut self, context: &str, question: &str, options: &[&str]) -> Option<Vec<f64>>;
}

/// What a [`Decide`] rule asks about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Sentence,
    /// A paragraph, heading or list item.
    Block,
}

impl Unit {
    pub fn parse(s: &str) -> Option<Unit> {
        match s {
            "sentence" => Some(Unit::Sentence),
            "block" | "paragraph" => Some(Unit::Block),
            _ => None,
        }
    }
}

/// A rule answered by the decision model: `question` about each unit, with
/// `options`; a unit is reported when option `flag` has probability
/// `threshold` or more.
#[derive(Debug, Clone)]
pub struct Decide {
    pub question: String,
    pub options: Vec<String>,
    pub flag: usize,
    pub threshold: f64,
    pub unit: Unit,
    /// Units with fewer words are not asked about.
    pub min_words: usize,
}

pub fn run_decide(d: &Decide, a: &Analysis, ask: &mut dyn Ask) -> Vec<Hit> {
    let options: Vec<&str> = d.options.iter().map(String::as_str).collect();
    let mut out = Vec::new();
    let mut flagged = |context: &str| -> Option<f64> {
        let p = ask.ask(context, &d.question, &options)?;
        let p = *p.get(d.flag)?;
        (p >= d.threshold).then_some(p)
    };
    match d.unit {
        Unit::Sentence => {
            for (si, s) in a.sentences.iter().enumerate() {
                if s.word_count() < d.min_words {
                    continue;
                }
                if let Some(p) = flagged(&s.original) {
                    let n = s.original.chars().count();
                    out.push(Hit::at(a, si, 0, n).var("percent", (p * 100.0).round()));
                }
            }
        }
        Unit::Block => {
            for (bi, b) in a.blocks.iter().enumerate() {
                if b.kind == BlockKind::Code || b.text.split_whitespace().count() < d.min_words {
                    continue;
                }
                let Some(first) = a.sentences.iter().position(|s| s.block == bi) else {
                    continue;
                };
                if let Some(p) = flagged(&b.text) {
                    let range = b.source_range(0..b.text.len());
                    out.push(
                        Hit::new(Some(first), range, b.text.trim())
                            .var("percent", (p * 100.0).round()),
                    );
                }
            }
        }
    }
    out
}

/// Character spans of a reading's phrases of two or more words.
fn spans(r: &Reading) -> HashSet<(usize, usize)> {
    let words: Vec<(usize, usize)> = r
        .nodes
        .iter()
        .filter(|n| n.leaf)
        .map(|n| (n.from, n.to))
        .collect();
    r.nodes
        .iter()
        .filter(|n| !n.leaf)
        .filter(|n| {
            words
                .iter()
                .filter(|&&(f, t)| f >= n.from && t <= n.to)
                .count()
                >= 2
        })
        .map(|n| (n.from, n.to))
        .collect()
}

/// The sentence with brackets around characters `[from, to)`, trailing
/// punctuation and spaces left outside.
fn bracket(sentence: &str, (from, to): (usize, usize)) -> String {
    let chars: Vec<char> = sentence.chars().collect();
    let mut to = to.min(chars.len());
    while to > from && (chars[to - 1].is_whitespace() || ",;:.!?".contains(chars[to - 1])) {
        to -= 1;
    }
    let mut s: String = chars[..from].iter().collect();
    s.push('[');
    s.extend(&chars[from..to]);
    s.push(']');
    s.extend(&chars[to..]);
    s
}

/// The shortest phrase of `a` that `b` does not have.
fn distinguishing(
    a: &HashSet<(usize, usize)>,
    b: &HashSet<(usize, usize)>,
) -> Option<(usize, usize)> {
    a.difference(b).copied().min_by_key(|&(f, t)| (t - f, f))
}

/// The two best readings of a sentence, if they score within `margin` of
/// each other and group the words differently: the shortest phrase of
/// each that the other does not have.
fn close_call(s: &crate::Sentence, margin: f64) -> Option<((usize, usize), (usize, usize))> {
    let readings = &s.parse.as_ref()?.readings;
    let (r0, r1) = (readings.first()?, readings.get(1)?);
    if (r0.score - r1.score).abs() > margin {
        return None;
    }
    let (s0, s1) = (spans(r0), spans(r1));
    Some((distinguishing(&s0, &s1)?, distinguishing(&s1, &s0)?))
}

/// The sentences before and after sentence `i`, as context for a question
/// about it.
fn surroundings(a: &Analysis, i: usize) -> String {
    let mut parts = Vec::new();
    if let Some(p) = i.checked_sub(1).and_then(|j| a.sentences.get(j)) {
        parts.push(format!("Before it: {}", p.original.trim()));
    }
    if let Some(n) = a.sentences.get(i + 1) {
        parts.push(format!("After it: {}", n.original.trim()));
    }
    parts.join("\n")
}

/// For sentences whose two best readings score within `margin` of each
/// other and group the words differently, ask which grouping is meant,
/// with the sentences around it as context, and put the second reading
/// first when its probability is at least `threshold`. Returns the number
/// of sentences whose best reading changed.
pub fn disambiguate(a: &mut Analysis, ask: &mut dyn Ask, margin: f64, threshold: f64) -> usize {
    let question = "Which grouping of words matches the meaning of the sentence?";
    let mut changed = 0;
    for i in 0..a.sentences.len() {
        let Some((d0, d1)) = close_call(&a.sentences[i], margin) else {
            continue;
        };
        let original = &a.sentences[i].original;
        let options = [bracket(original, d0), bracket(original, d1)];
        if options[0] == options[1] {
            continue;
        }
        let context = surroundings(a, i);
        let opts: Vec<&str> = options.iter().map(String::as_str).collect();
        let Some(p) = ask.ask(&context, question, &opts) else {
            continue;
        };
        if p.get(1).is_some_and(|&p1| p1 >= threshold) {
            if let Some(parse) = a.sentences[i].parse.as_mut() {
                parse.readings.swap(0, 1);
                changed += 1;
            }
        }
    }
    changed
}

/// Lower-case words of characters `[from, to)` of a sentence, without
/// punctuation at the edges.
fn phrase_text(sentence: &str, (from, to): (usize, usize)) -> String {
    let chars: Vec<char> = sentence.chars().collect();
    let text: String = chars[from.min(chars.len())..to.min(chars.len())]
        .iter()
        .collect();
    text.split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Settle close calls from the rest of the document. Among the readings of
/// a sentence that score within `margin` of the best (at most ten), the one
/// with the most phrases of two or more content words that also occur
/// elsewhere as a phrase (in the best reading of another sentence, or as a
/// whole heading) becomes the best;
/// phrases every candidate shares do not count, and ties keep the parser's
/// order. A document headed "Sleep and Memory" supports "the relationship
/// between [sleep and memory]". Returns the number of sentences changed.
pub fn prefer_document_phrases(a: &mut Analysis, margin: f64) -> usize {
    use std::collections::HashMap;
    // Phrase -> sentences (or headings) it occurs in.
    let mut seen: HashMap<String, HashSet<usize>> = HashMap::new();
    for (i, s) in a.sentences.iter().enumerate() {
        if let Some(r) = s.best() {
            for sp in spans(r) {
                seen.entry(phrase_text(&s.original, sp))
                    .or_default()
                    .insert(i);
            }
        }
        if matches!(a.blocks[s.block].kind, BlockKind::Heading(_)) {
            let n = s.original.chars().count();
            seen.entry(phrase_text(&s.original, (0, n)))
                .or_default()
                .insert(i);
        }
    }
    // Phrases of fewer than two content words ("of them") are too common to
    // say anything.
    const FUNCTION_WORDS: &[&str] = &[
        "a", "an", "the", "of", "to", "in", "on", "at", "by", "for", "with", "and", "or", "but",
        "as", "is", "are", "was", "were", "be", "it", "its", "they", "them", "their", "he", "him",
        "his", "she", "her", "we", "us", "our", "you", "your", "i", "me", "my", "that", "this",
        "these", "those", "not", "so", "too", "very", "just", "one", "all",
    ];
    let elsewhere = |text: &str, i: usize| {
        text.split(' ')
            .filter(|w| !FUNCTION_WORDS.contains(w))
            .count()
            >= 2
            && seen.get(text).is_some_and(|at| at.iter().any(|&j| j != i))
    };
    let mut changed = 0;
    for i in 0..a.sentences.len() {
        let Some(parse) = a.sentences[i].parse.as_ref() else {
            continue;
        };
        let Some(top) = parse.readings.first().map(|r| r.score) else {
            continue;
        };
        let candidates: Vec<HashSet<(usize, usize)>> = parse
            .readings
            .iter()
            .take(10)
            .take_while(|r| top - r.score <= margin)
            .map(spans)
            .collect();
        if candidates.len() < 2 {
            continue;
        }
        let shared: HashSet<(usize, usize)> = candidates
            .iter()
            .skip(1)
            .fold(candidates[0].clone(), |acc, c| {
                acc.intersection(c).copied().collect()
            });
        let original = &a.sentences[i].original;
        let support: Vec<usize> = candidates
            .iter()
            .map(|c| {
                c.difference(&shared)
                    .filter(|&&sp| elsewhere(&phrase_text(original, sp), i))
                    .count()
            })
            .collect();
        let best = (0..support.len())
            .max_by_key(|&k| (support[k], std::cmp::Reverse(k)))
            .unwrap_or(0);
        if best > 0 && support[best] > support[0] {
            if let Some(parse) = a.sentences[i].parse.as_mut() {
                let r = parse.readings.remove(best);
                parse.readings.insert(0, r);
                changed += 1;
            }
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brackets() {
        assert_eq!(
            bracket("The relationship between sleep and memory.", (4, 30)),
            "The [relationship between sleep] and memory."
        );
        assert_eq!(bracket("Sleep and memory.", (0, 17)), "[Sleep and memory].");
    }
}
