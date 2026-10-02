//! Modifiers: adjectives piled on one noun ("the vibrant, confident and
//! independent women") and sentences dense with adjectives and adverbs,
//! from the grammar's analysis. Professional editors cut both from
//! machine-written prose (they are among the "purple prose" edits in the
//! LAMP study, Chakrabarty et al. 2025).

use emdysi_parse::{Reading, Word};

use crate::Analysis;
use crate::structure::Hit;

/// Whether a word is an adjective or a participle used as one, not a
/// number or ordinal.
fn adjectival(w: &Word) -> bool {
    let numeric = w.le_type.contains("card") || w.le_type.contains("ord");
    (w.le_type.starts_with("aj_") && !numeric) || w.rules.iter().any(|r| r.starts_with("v_j-"))
}

/// Adverbs that do not describe: negation, focus and degree words, and
/// sentence connectives.
const PLAIN_ADVERBS: &[&str] = &[
    "not",
    "n't",
    "also",
    "only",
    "just",
    "too",
    "very",
    "so",
    "even",
    "then",
    "now",
    "still",
    "there",
    "here",
    "more",
    "most",
    "less",
    "least",
    "as",
    "how",
    "when",
    "why",
    "where",
    "again",
    "already",
    "always",
    "never",
    "often",
    "however",
    "therefore",
    "thus",
    "yet",
    "ever",
    "soon",
    "later",
    "today",
    "together",
    "back",
    "away",
    "up",
    "down",
    "out",
    "off",
    "almost",
    "enough",
    "rather",
    "quite",
];

fn descriptive_adverb(w: &Word) -> bool {
    w.le_type.starts_with("av_") && !PLAIN_ADVERBS.contains(&w.surface.to_lowercase().as_str())
}

fn is_modifier_node(name: &str) -> bool {
    name.starts_with("aj-hdn_")
}

/// Noun phrases whose noun carries `min` or more adjectives.
pub fn run_adjective_stacks(a: &Analysis, min: usize) -> Vec<Hit> {
    let mut out = Vec::new();
    for (si, s) in a.sentences.iter().enumerate() {
        let Some(r) = s.best() else { continue };
        for (ni, n) in r.nodes.iter().enumerate() {
            if n.leaf || !is_modifier_node(&n.name) {
                continue;
            }
            // Only the outermost modifier of a chain.
            if let Some(p) = n.parent {
                let pn = &r.nodes[p];
                if is_modifier_node(&pn.name) && pn.children.last() == Some(&ni) {
                    continue;
                }
            }
            let (count, from) = chain(r, ni);
            if count >= min {
                let text: String = s.original.chars().skip(from).take(n.to - from).collect();
                let trimmed = text.trim_end_matches(|c: char| !c.is_alphanumeric());
                let to = from + trimmed.chars().count();
                out.push(Hit::at(a, si, from, to).var("count", count));
            }
        }
    }
    out
}

/// Adjectives along a chain of modifier nodes, and where the first one
/// starts.
fn chain(r: &Reading, top: usize) -> (usize, usize) {
    let mut count = 0;
    let mut cur = top;
    let start = r.nodes[top].from;
    while is_modifier_node(&r.nodes[cur].name) {
        let n = &r.nodes[cur];
        let (Some(&left), Some(&head)) = (n.children.first(), n.children.last()) else {
            break;
        };
        let (lf, lt) = (r.nodes[left].from, r.nodes[left].to);
        count += r
            .words
            .iter()
            .filter(|w| w.from >= lf && w.to <= lt && adjectival(w))
            .count();
        cur = head;
    }
    (count, start)
}

/// Sentences in which adjectives and descriptive adverbs make up at least
/// `ratio` of the words, and number at least `min`.
pub fn run_modifier_density(a: &Analysis, min: usize, ratio: f64) -> Vec<Hit> {
    let mut out = Vec::new();
    for (si, s) in a.sentences.iter().enumerate() {
        let Some(r) = s.best() else { continue };
        let words = s.word_count();
        if words == 0 {
            continue;
        }
        let n = r
            .words
            .iter()
            .filter(|w| adjectival(w) || descriptive_adverb(w))
            .count();
        if n >= min && n as f64 >= ratio * words as f64 {
            let len = s.original.chars().count();
            out.push(
                Hit::at(a, si, 0, len)
                    .var("count", n)
                    .var("percent", (100.0 * n as f64 / words as f64).round()),
            );
        }
    }
    out
}
