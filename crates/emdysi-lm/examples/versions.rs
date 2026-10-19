//! How a choice among versions of a sentence trades corrections against
//! false corrections, by the margin a correction must win by. Reads the
//! questions `real_errors` writes with `LOG` (FILE.versions.tsv: question,
//! context, versions separated by '|' with the original first, index of
//! the right one, 0 when the original is right).
//!
//! `cargo run --release -p emdysi-lm --features llama --example versions -- SPEC FILE.tsv`
//!
//! Each version is scored once; then, for each way of comparing them and
//! each margin (in nats), a correction is made when the best version beats
//! the original by the margin or more. Reported: real errors corrected
//! right, corrected wrong (another version than the writer's), and
//! questions whose original is right that were changed anyway.
//!
//! - `plain`: the log-probability of the version after the context;
//! - `normalized`: less the log-probability of its words alone (SLOR);
//! - `per token`: the plain log-probability per token.

use emdysi_lm::LanguageModel;

/// The scores of a question's versions by each way of comparing them.
struct Scored {
    right: usize,
    plain: Vec<f64>,
    normalized: Vec<f64>,
    per_token: Vec<f64>,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [spec, file] = &args[..] else {
        eprintln!("usage: versions SPEC FILE.tsv");
        std::process::exit(2);
    };
    let mut lm = emdysi_lm::open(spec).unwrap();
    let src = std::fs::read_to_string(file).unwrap();
    let mut all = Vec::new();
    for line in src.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        let [_, context, options, right] = f[..] else {
            continue;
        };
        let Ok(right) = right.parse::<usize>() else {
            continue;
        };
        let options: Vec<&str> = options.split('|').collect();
        if let Some(s) = score(&mut *lm, context, &options, right) {
            all.push(s);
        }
    }
    let errors = all.iter().filter(|s| s.right > 0).count();
    let clean = all.len() - errors;
    println!(
        "{} questions: {errors} with a right correction, {clean} with the original right",
        all.len()
    );
    for (name, get) in [
        ("plain", (|s: &Scored| &s.plain) as fn(&Scored) -> &Vec<f64>),
        ("normalized", |s: &Scored| &s.normalized),
        ("per token", |s: &Scored| &s.per_token),
    ] {
        println!(
            "\n{name}\n| Margin | Corrected right | Corrected wrong | Original right but changed |\n|---|---|---|---|"
        );
        let margins: &[f64] = if name == "per token" {
            &[0.0, 0.1, 0.2, 0.3, 0.5, 0.75, 1.0]
        } else {
            &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0]
        };
        for &m in margins {
            let (mut right, mut wrong, mut changed) = (0, 0, 0);
            for s in &all {
                let v = get(s);
                let best = (1..v.len()).max_by(|&a, &b| v[a].total_cmp(&v[b])).unwrap();
                if v[best] - v[0] < m {
                    continue;
                }
                match s.right {
                    0 => changed += 1,
                    r if r == best => right += 1,
                    _ => wrong += 1,
                }
            }
            println!("| {m} | {right} of {errors} | {wrong} | {changed} of {clean} |");
        }
    }
}

fn score(
    lm: &mut dyn LanguageModel,
    context: &str,
    options: &[&str],
    right: usize,
) -> Option<Scored> {
    let prefix = if context.trim().is_empty() {
        String::new()
    } else {
        format!("{} ", context.trim())
    };
    let mut plain = Vec::new();
    let mut per_token = Vec::new();
    for o in options {
        let (lp, n) = lm.logprob(&prefix, o.trim())?;
        plain.push(lp);
        per_token.push(lp / n.max(1) as f64);
    }
    // As `Decider::raw` normalizes: plus the surprise of the words alone.
    let mut words: Vec<&str> = options.iter().flat_map(|o| o.split_whitespace()).collect();
    words.sort_unstable();
    words.dedup();
    let cost = emdysi_lm::decide::alone(lm, &words)?;
    let normalized = options
        .iter()
        .zip(&plain)
        .map(|(o, lp)| {
            lp + o
                .split_whitespace()
                .map(|w| cost[words.binary_search(&w).unwrap_or(0)])
                .sum::<f64>()
        })
        .collect();
    Some(Scored {
        right,
        plain,
        normalized,
        per_token,
    })
}
