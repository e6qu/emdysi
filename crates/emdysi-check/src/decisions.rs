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
//!   a sentence by showing the model the sentence bracketed both ways.

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

/// For sentences whose two best readings score within `margin` of each
/// other and group the words differently, ask which grouping is meant, and
/// put the second reading first when its probability is at least
/// `threshold`. Returns the number of sentences whose best reading changed.
pub fn disambiguate(a: &mut Analysis, ask: &mut dyn Ask, margin: f64, threshold: f64) -> usize {
    let question = "Which grouping of words matches the meaning of the sentence?";
    let mut changed = 0;
    for s in &mut a.sentences {
        let Some(parse) = s.parse.as_mut() else {
            continue;
        };
        if parse.readings.len() < 2 {
            continue;
        }
        let (r0, r1) = (&parse.readings[0], &parse.readings[1]);
        if (r0.score - r1.score).abs() > margin {
            continue;
        }
        let (s0, s1) = (spans(r0), spans(r1));
        let (Some(d0), Some(d1)) = (distinguishing(&s0, &s1), distinguishing(&s1, &s0)) else {
            continue;
        };
        let options = [bracket(&s.original, d0), bracket(&s.original, d1)];
        if options[0] == options[1] {
            continue;
        }
        let opts: Vec<&str> = options.iter().map(String::as_str).collect();
        let Some(p) = ask.ask("", question, &opts) else {
            continue;
        };
        if p.get(1).is_some_and(|&p1| p1 >= threshold) {
            parse.readings.swap(0, 1);
            changed += 1;
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
