//! Rule-based sentence segmentation.
//!
//! The text is split into whitespace-separated chunks, and a boundary is
//! placed after a chunk ending in terminal punctuation unless the context
//! says otherwise: abbreviations and initials, ellipses inside a sentence,
//! a lower-case continuation, and so on. Enumerated lists (`1.`, `2)`,
//! `a.`, bullets) start a new sentence at each item.

use std::ops::Range;

/// Abbreviations that precede a name or number and never end a sentence.
const TITLES: &[&str] = &[
    "mr", "mrs", "ms", "mx", "dr", "prof", "rev", "hon", "gov", "pres", "sen", "rep", "gen", "col",
    "lt", "sgt", "capt", "cmdr", "adm", "maj", "st", "mt", "ft", "no", "nos", "nr", "fig", "figs",
    "vol", "vols", "pp", "p", "ch", "sec", "eq", "art", "op", "cf", "vs", "ca", "approx", "dept",
    "est", "jan", "feb", "mar", "apr", "jun", "jul", "aug", "sep", "sept", "oct", "nov", "dec",
    "n°",
];

/// Other common abbreviations: they end a sentence only when a typical
/// sentence opener follows.
const ABBREVIATIONS: &[&str] = &[
    "etc", "al", "co", "corp", "inc", "ltd", "llc", "jr", "sr", "bros", "dept", "univ", "assn",
    "ave", "blvd", "rd", "e.g", "i.e", "viz", "resp", "incl", "misc", "min", "max", "hr", "hrs",
    "mins", "sec", "secs",
];

/// Words that typically open a sentence, used to decide whether an
/// abbreviation also ends one.
const OPENERS: &[&str] = &[
    "i", "you", "he", "she", "it", "we", "they", "the", "a", "an", "this", "that", "these",
    "those", "there", "here", "how", "what", "why", "when", "where", "who", "which", "but", "and",
    "so", "then", "if", "in", "on", "at", "after", "before", "however", "please", "my", "our",
    "his", "her", "their", "its", "your", "mr", "mrs", "ms", "dr", "did", "do", "does", "is",
    "are", "was", "were", "can", "could", "will", "would", "should", "let",
];

const CLOSERS: &[char] = &[')', ']', '}', '”', '’', '"', '\'', '»', '›'];
const OPENERS_PUNCT: &[char] = &['(', '[', '{', '“', '‘', '"', '\'', '«', '‹', '¿', '¡'];

#[derive(Debug, Clone)]
struct Chunk {
    start: usize,
    end: usize,
}

fn chunks(text: &str) -> Vec<Chunk> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, c) in text.char_indices() {
        if c.is_whitespace() {
            if let Some(s) = start.take() {
                out.push(Chunk { start: s, end: i });
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(s) = start {
        out.push(Chunk {
            start: s,
            end: text.len(),
        });
    }
    out
}

/// A list marker such as `1.`, `2.)`, `3)`, `b.`, `•`, `⁃10.`, as an ordinal.
fn list_marker(w: &str) -> Option<(char, u32)> {
    let w = w.trim_start_matches(['•', '⁃', '-', '*']);
    let core = w
        .strip_suffix(".)")
        .or_else(|| w.strip_suffix(')'))
        .or_else(|| w.strip_suffix('.'))?;
    if !core.is_empty() && core.chars().all(|c| c.is_ascii_digit()) && core.len() <= 3 {
        return core.parse().ok().map(|n| ('1', n));
    }
    let mut cs = core.chars();
    match (cs.next(), cs.next()) {
        (Some(c), None) if c.is_ascii_lowercase() => Some(('a', c as u32 - 'a' as u32 + 1)),
        _ => None,
    }
}

fn starts_upper_or_open(w: &str) -> bool {
    let w = w.trim_start_matches(OPENERS_PUNCT);
    w.chars()
        .next()
        .is_some_and(|c| c.is_uppercase() || c.is_ascii_digit())
}

fn first_word_lower(w: &str) -> String {
    w.trim_start_matches(OPENERS_PUNCT)
        .trim_end_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

/// Is `w` (including its final period) an abbreviation, and if so, a title
/// that never ends a sentence?
fn abbreviation(w: &str) -> Option<bool> {
    let core = w.strip_suffix('.')?;
    let core = core.trim_start_matches(OPENERS_PUNCT);
    let lower = core.to_lowercase();
    if TITLES.contains(&lower.as_str()) {
        return Some(true);
    }
    if ABBREVIATIONS.contains(&lower.as_str()) {
        return Some(false);
    }
    // Dotted abbreviations: U.S, e.g, a.m, U.S.A
    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() > 1
        && parts
            .iter()
            .all(|p| p.chars().count() == 1 && p.chars().all(char::is_alphabetic))
    {
        return Some(false);
    }
    // Single-letter initials: J. K. Rowling.
    let mut cs = core.chars();
    if let (Some(c), None) = (cs.next(), cs.next()) {
        if c.is_uppercase() {
            return Some(core != "I");
        }
    }
    None
}

/// Sentence spans in `text` (byte ranges, without surrounding whitespace).
pub fn sentences(text: &str) -> Vec<Range<usize>> {
    let ch = chunks(text);
    if ch.is_empty() {
        return Vec::new();
    }
    let word = |i: usize| &text[ch[i].start..ch[i].end];
    let mut breaks = vec![false; ch.len()];

    // Enumerated lists: each next marker in sequence starts a sentence. A
    // bullet may be a chunk of its own (`• 9.`).
    let is_bullet = |w: &str| matches!(w, "•" | "⁃" | "-" | "*" | "–");
    let mut markers = vec![false; ch.len()];
    let first = usize::from(is_bullet(word(0)) && ch.len() > 1);
    if let Some((kind, mut n)) = list_marker(word(first)) {
        markers[first] = true;
        let mut item_breaks = Vec::new();
        for (k, marker) in markers.iter_mut().enumerate().skip(first + 1) {
            if let Some((kd, m)) = list_marker(word(k)) {
                if kd == kind && m == n + 1 {
                    *marker = true;
                    let start = if k > 0 && is_bullet(word(k - 1)) {
                        k - 1
                    } else {
                        k
                    };
                    item_breaks.push(start);
                    n = m;
                }
            }
        }
        for b in item_breaks {
            breaks[b] = true;
        }
    }

    let mut i = 0;
    while i + 1 < ch.len() {
        let w = word(i);
        let next = word(i + 1);
        // Spaced ellipses: ". . ." inside a sentence, ". . . ." at its end.
        if w == "." {
            let mut j = i;
            while j < ch.len() && word(j) == "." {
                j += 1;
            }
            let dots = j - i;
            if j < ch.len() && dots >= 4 && starts_upper_or_open(word(j)) {
                breaks[j] = true;
            }
            i = j;
            continue;
        }
        if breaks[i + 1] || markers[i] {
            i += 1;
            continue;
        }
        let core = w.trim_end_matches(CLOSERS);
        let closed = core.len() < w.len();
        let Some(last) = core.chars().last() else {
            i += 1;
            continue;
        };
        let next_upper = starts_upper_or_open(next);
        let brk = match last {
            '!' | '?' | '‼' | '⁇' => next_upper,
            '…' => next_upper && !core.ends_with(" …"),
            '.' => {
                if core.ends_with("...") || core.ends_with("..") {
                    // Attached ellipsis: a following capital starts a sentence
                    // when the dots are not followed by a lower-case word.
                    next_upper && core.ends_with("....") || next_upper && !core.ends_with("...")
                } else if !next_upper {
                    false
                } else if closed {
                    true
                } else {
                    match abbreviation(core) {
                        Some(true) => false,
                        Some(false) => {
                            let n = first_word_lower(next);
                            OPENERS.contains(&n.as_str())
                                && abbreviation(next).is_none_or(|title| !title)
                                || abbreviation(next) == Some(true)
                                    && !core.chars().next().is_some_and(char::is_lowercase)
                        }
                        None => {
                            // "you and I. Did" ends a sentence; "Albert I. Jones" does not.
                            if core == "I." {
                                i > 0 && word(i - 1).chars().next().is_some_and(char::is_lowercase)
                            } else {
                                !(next
                                    .trim_start_matches(OPENERS_PUNCT)
                                    .starts_with(char::is_numeric)
                                    && core.ends_with("°."))
                            }
                        }
                    }
                }
            }
            _ => false,
        };
        if brk {
            breaks[i + 1] = true;
        }
        i += 1;
    }

    let mut out = Vec::new();
    let mut start = ch[0].start;
    for k in 1..ch.len() {
        if breaks[k] {
            out.push(start..ch[k - 1].end);
            start = ch[k].start;
        }
    }
    out.push(start..ch[ch.len() - 1].end);
    out
}
