//! How well the model's surprisal at each word finds errors, on minimal
//! pairs (an `en decide --eval` file of two options, a sentence as written
//! and as corrected, such as `scripts/model-eval/questions.py` writes).
//!
//! `cargo run --release -p emdysi-lm --features llama --example surprisal -- SPEC FILE.tsv`
//!
//! A word's surprise is `Decider::surprise`: the negative log-probability
//! of its tokens where it stands, less that of the word alone (with `RAW`,
//! without that correction). Reported:
//!
//! - locating: how often the most surprising word of the sentence as
//!   written is the corrected word (for an insertion, the word after the
//!   gap), and how often it is among the three most surprising;
//! - screening: how well the highest word surprisal of a sentence tells the
//!   sentence as written from the corrected one (the area under the ROC
//!   curve: 0.5 is chance), with and without the first word, whose
//!   surprisal without context is high whatever the sentence.
//!
//! `SHOW` prints each sentence as written with its words' surprise.

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [spec, file] = &args[..] else {
        eprintln!("usage: surprisal SPEC FILE.tsv");
        std::process::exit(2);
    };
    let mut lm = emdysi_lm::open(spec).unwrap();
    let src = std::fs::read_to_string(file).unwrap();
    let (mut n, mut top1, mut top3) = (0, 0, 0);
    // (highest word surprisal, the same without the first word) of each
    // sentence as written and as corrected.
    let mut wrong: Vec<(f64, f64)> = Vec::new();
    let mut right: Vec<(f64, f64)> = Vec::new();
    for line in src.lines() {
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
        let (Some(wb), Some(wg)) = (words(&mut *lm, bad), words(&mut *lm, good)) else {
            continue;
        };
        let peak = |w: &[(usize, usize, f64)]| {
            let all = w.iter().map(|x| x.2).fold(0.0, f64::max);
            let rest = w.iter().skip(1).map(|x| x.2).fold(0.0, f64::max);
            (all, rest)
        };
        if std::env::var_os("SHOW").is_some() {
            for w in &wb {
                eprint!("{}={:.1} ", &bad[w.0..w.1], w.2);
            }
            eprintln!();
        }
        wrong.push(peak(&wb));
        right.push(peak(&wg));
        // The corrected word: where the two first differ (for an
        // insertion, the word after the gap).
        let p = bad
            .bytes()
            .zip(good.bytes())
            .take_while(|(a, b)| a == b)
            .count();
        let Some(k) = wb.iter().position(|&(_, to, _)| p < to) else {
            continue;
        };
        let mut order: Vec<usize> = (0..wb.len()).collect();
        order.sort_by(|&a, &b| wb[b].2.total_cmp(&wb[a].2));
        n += 1;
        top1 += usize::from(order[0] == k);
        top3 += usize::from(order.iter().take(3).any(|&i| i == k));
    }
    println!(
        "locating: {n} sentences; most surprising word right {top1} ({:.1}%), among the three most {top3} ({:.1}%)",
        100.0 * top1 as f64 / n.max(1) as f64,
        100.0 * top3 as f64 / n.max(1) as f64
    );
    let w0: Vec<f64> = wrong.iter().map(|x| x.0).collect();
    let r0: Vec<f64> = right.iter().map(|x| x.0).collect();
    let w1: Vec<f64> = wrong.iter().map(|x| x.1).collect();
    let r1: Vec<f64> = right.iter().map(|x| x.1).collect();
    println!(
        "screening: {} pairs; AUC of the highest word surprisal {:.3}, without the first word {:.3}",
        wrong.len(),
        auc(&w0, &r0),
        auc(&w1, &r1)
    );
    // How many sentences as written and as corrected reach each highest
    // surprisal (without the first word): a screen's recall and its cost.
    for t in [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0] {
        let share = |v: &[f64]| {
            100.0 * v.iter().filter(|&&x| x >= t).count() as f64 / v.len().max(1) as f64
        };
        println!(
            "  {t:>4.1} nats or more: {:.0}% of sentences as written, {:.0}% as corrected",
            share(&w1),
            share(&r1)
        );
    }
}

/// The words of `text` (runs of non-space characters) as (start, end,
/// surprise): `Decider::surprise`, or with `RAW` the plain negative
/// log-probability of the word's tokens.
fn words(lm: &mut dyn emdysi_lm::LanguageModel, text: &str) -> Option<Vec<(usize, usize, f64)>> {
    let mut spans = Vec::new();
    let mut at = 0;
    for w in text.split_whitespace() {
        let from = at + text[at..].find(w)?;
        at = from + w.len();
        spans.push((from, at));
    }
    let scores = if std::env::var_os("RAW").is_some() {
        let mut out = vec![0.0; spans.len()];
        for (at, lp) in lm.token_logprobs("", text)? {
            let rest = text.get(at..).unwrap_or("");
            let at = at + rest.len() - rest.trim_start().len();
            if let Some(k) = spans.iter().position(|&(f, t)| f <= at && at < t) {
                out[k] -= lp;
            }
        }
        out
    } else {
        emdysi_lm::decide::Decider::new(lm)
            .surprise("", text, &spans)
            .ok()?
    };
    Some(
        spans
            .iter()
            .zip(scores)
            .map(|(&(f, t), x)| (f, t, x))
            .collect(),
    )
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
