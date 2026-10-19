//! How well the model's own expectations find errors, on minimal pairs (an
//! `en decide --eval` file of two options, a sentence as written and as
//! corrected, such as `scripts/model-eval/questions.py` writes); compare
//! `examples/surprisal`.
//!
//! `cargo run --release -p emdysi-lm --features llama --example alternatives -- SPEC FILE.tsv`
//!
//! At each token, the gain of an edit is how much likelier the model
//! finds something else there than the token itself: a function word
//! (inserted before the token, or in its place) or the token after it
//! (this one deleted), in nats. A word's gain is the highest at its
//! tokens; a sentence's, the highest of its words'. An alternative counts
//! only where the model expects it with some probability (the floors:
//! at a rare word, every function word is likelier than the word itself).
//! Reported, per floor, as for
//! surprise: how often the word with the highest gain is the corrected
//! one, and how well a sentence's gain tells the sentence as written from
//! the corrected one.

/// Function words writers drop, add or confuse (as `assist` edits them).
const WORDS: &[&str] = &[
    "a", "an", "the", "they", "then", "is", "are", "was", "were", "be", "do", "does", "did", "has",
    "have", "had", "can", "could", "should", "will", "would", "may", "must", "in", "on", "at",
    "for", "to", "of", "from", "by", "with", "into", "about", "as", "within", "it", "its", "them",
    "their", "your", "our", "his", "her", "you", "we", "this", "that", "which", "and", "or", "not",
];

/// The least probability, at a token, of the expected word or next token
/// for its gain to count: at a rare word every function word is likelier
/// than the word itself, which says nothing.
const FLOORS: [f64; 5] = [0.0, 0.05, 0.1, 0.25, 0.5];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [spec, file] = &args[..] else {
        eprintln!("usage: alternatives SPEC FILE.tsv");
        std::process::exit(2);
    };
    let mut lm = emdysi_lm::open(spec).unwrap();
    let src = std::fs::read_to_string(file).unwrap();
    // Per floor: located (n, top1, top3), and the sentences' gains.
    let mut located = [(0, 0, 0); FLOORS.len()];
    let mut wrong: Vec<Vec<f64>> = vec![Vec::new(); FLOORS.len()];
    let mut right: Vec<Vec<f64>> = vec![Vec::new(); FLOORS.len()];
    for (k, line) in src.lines().enumerate() {
        let f: Vec<&str> = line.split('\t').collect();
        let [_, _, options, answer] = f[..] else {
            continue;
        };
        let options: Vec<&str> = options.split('|').collect();
        let Ok(answer) = answer.parse::<usize>() else {
            continue;
        };
        if options.len() != 2 {
            continue;
        }
        let (bad, good) = (options[1 - answer], options[answer]);
        let (Some(wb), Some(wg)) = (gains(&mut *lm, bad), gains(&mut *lm, good)) else {
            continue;
        };
        let p = bad
            .bytes()
            .zip(good.bytes())
            .take_while(|(a, b)| a == b)
            .count();
        for f in 0..FLOORS.len() {
            let peak = |w: &[(usize, usize, Vec<f64>)]| {
                w.iter().map(|x| x.2[f]).fold(f64::NEG_INFINITY, f64::max)
            };
            wrong[f].push(peak(&wb));
            right[f].push(peak(&wg));
            if let Some(i) = wb.iter().position(|&(_, to, _)| p < to) {
                let mut order: Vec<usize> = (0..wb.len()).collect();
                order.sort_by(|&a, &b| wb[b].2[f].total_cmp(&wb[a].2[f]));
                let l = &mut located[f];
                l.0 += 1;
                l.1 += usize::from(order[0] == i && wb[i].2[f] > f64::NEG_INFINITY);
                l.2 += usize::from(
                    order.iter().take(3).any(|&j| j == i) && wb[i].2[f] > f64::NEG_INFINITY,
                );
            }
        }
        if k % 50 == 49 {
            eprintln!("{} pairs", k + 1);
        }
    }
    for (f, floor) in FLOORS.iter().enumerate() {
        let (n, top1, top3) = located[f];
        println!(
            "\nexpected word at least {floor}: located {top1} of {n} ({:.1}%), among the three highest {top3} ({:.1}%); AUC {:.3}",
            100.0 * top1 as f64 / n.max(1) as f64,
            100.0 * top3 as f64 / n.max(1) as f64,
            auc(&wrong[f], &right[f])
        );
        for t in [0.0, 1.0, 2.0, 3.0, 4.0, 6.0] {
            let share = |v: &[f64]| {
                100.0 * v.iter().filter(|&&x| x >= t).count() as f64 / v.len().max(1) as f64
            };
            println!(
                "  gain {t:>3.1} nats or more: {:.0}% of sentences as written, {:.0}% as corrected",
                share(&wrong[f]),
                share(&right[f])
            );
        }
    }
}

/// The words of `text` (runs of non-space characters) as (start, end,
/// gain per floor): the highest gain at the tokens that start in the
/// word, counting an alternative only if its probability reaches the
/// floor.
fn gains(
    lm: &mut dyn emdysi_lm::LanguageModel,
    text: &str,
) -> Option<Vec<(usize, usize, Vec<f64>)>> {
    let mut words = Vec::new();
    let mut at = 0;
    for w in text.split_whitespace() {
        let from = at + text[at..].find(w)?;
        at = from + w.len();
        words.push((from, at, vec![f64::NEG_INFINITY; FLOORS.len()]));
    }
    for a in lm.alternatives("", text, WORDS)? {
        let rest = text.get(a.at..).unwrap_or("");
        let at = a.at + rest.len() - rest.trim_start().len();
        let Some(w) = words.iter_mut().find(|w| w.0 <= at && at < w.1) else {
            continue;
        };
        for alt in a.words.iter().copied().chain(a.next) {
            for (f, floor) in FLOORS.iter().enumerate() {
                if alt >= floor.ln() {
                    w.2[f] = w.2[f].max(alt - a.actual);
                }
            }
        }
    }
    Some(words)
}

/// The probability that a score from `pos` is higher than one from `neg`
/// (ties count half).
fn auc(pos: &[f64], neg: &[f64]) -> f64 {
    let mut s = 0.0;
    for p in pos {
        for q in neg {
            s += if p > q {
                1.0
            } else if p == q {
                0.5
            } else {
                0.0
            };
        }
    }
    s / (pos.len() * neg.len()).max(1) as f64
}
