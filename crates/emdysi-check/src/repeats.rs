//! Words written twice by mistake: a function word doubled ("the the",
//! "to to") or two different articles in a row ("a the"). English allows
//! some doublings ("that that is is", "Will Will will", "had had"), so a
//! doubling is reported only in a sentence the grammar has no full analysis
//! for, and a run of three or more is taken as deliberate ("Buffalo buffalo
//! buffalo", "can can can").

use crate::Analysis;
use crate::structure::Hit;

/// Function words that are checked for doubling.
const WORDS: &[&str] = &[
    "the", "a", "an", "to", "of", "for", "with", "from", "into", "and", "or", "but", "are", "be",
    "been", "can", "will", "would", "should", "could", "we", "they", "this", "is",
];

const ARTICLES: &[&str] = &["a", "an", "the"];

/// Repeated words (`articles` false) or two different articles in a row
/// (`articles` true).
pub fn run_repeats(a: &Analysis, articles: bool) -> Vec<Hit> {
    let mut out = Vec::new();
    for (si, s) in a.sentences.iter().enumerate() {
        if s.strict() {
            continue;
        }
        let t = &s.tokens;
        for i in 0..t.len().saturating_sub(1) {
            let (x, y) = (&t[i], &t[i + 1]);
            let gap: String = s
                .original
                .chars()
                .skip(x.to)
                .take(y.from.saturating_sub(x.to))
                .collect();
            if y.from <= x.to || !gap.chars().all(|c| c == ' ' || c == '\t') {
                continue;
            }
            let (lx, ly) = (x.form.to_lowercase(), y.form.to_lowercase());
            let same = |k: usize| t.get(k).is_some_and(|w| w.form.to_lowercase() == lx);
            let hit = if articles {
                // Lower case, so that "the A record" is not read as two
                // articles.
                lx != ly
                    && ARTICLES.contains(&x.form.as_str())
                    && ARTICLES.contains(&y.form.as_str())
            } else {
                lx == ly && WORDS.contains(&lx.as_str()) && !(i > 0 && same(i - 1)) && !same(i + 2)
            };
            if hit {
                let mut h = Hit::at(a, si, x.from, y.to);
                if !articles {
                    h.replacement = Some(x.form.clone());
                }
                out.push(h);
            }
        }
    }
    out
}
