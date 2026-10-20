//! Whether one pass over a sentence can pick out, among its versions, the
//! few worth comparing in full: for the versions questions `real_errors`
//! saves (`LOG.versions.tsv`: question, context, versions separated by
//! '|' with the original first, index of the right one), how often the
//! writer's version is among the first k by an estimate from the
//! original's own pass (`LanguageModel::alternatives`).
//!
//! `cargo run --release -p emdysi-lm --features llama --example prefilter -- SPEC FILE.tsv...`
//!
//! A version's estimate is `Decider::estimates`. Only questions whose
//! right version is not the original are counted.

use emdysi_lm::LanguageModel;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((spec, files)) = args.split_first() else {
        eprintln!("usage: prefilter SPEC FILE.tsv...");
        std::process::exit(2);
    };
    let mut lm = emdysi_lm::open(spec).unwrap();
    // Rank of the right version by the estimate (0 = first), and the
    // number of versions, per question.
    let mut ranks: Vec<(usize, usize)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for file in files {
        for line in std::fs::read_to_string(file).unwrap().lines() {
            let f: Vec<&str> = line.split('\t').collect();
            let [_, context, options, right] = f[..] else {
                continue;
            };
            let Ok(right) = right.parse::<usize>() else {
                continue;
            };
            if right == 0 || !seen.insert((context.to_string(), options.to_string())) {
                continue;
            }
            let options: Vec<&str> = options.split('|').collect();
            if let Some(est) = estimates(&mut *lm, context, &options) {
                let r = est[right];
                let rank = (1..est.len()).filter(|&i| i != right && est[i] > r).count();
                ranks.push((rank, est.len() - 1));
            }
        }
    }
    let n = ranks.len();
    println!(
        "{n} questions with the writer's version among others (on average {:.1} versions)",
        ranks.iter().map(|r| r.1).sum::<usize>() as f64 / n.max(1) as f64
    );
    for k in [1, 2, 3, 5, 9] {
        let hit = ranks.iter().filter(|r| r.0 < k).count();
        println!(
            "  among the first {k}: {hit} ({:.0}%)",
            100.0 * hit as f64 / n.max(1) as f64
        );
    }
}

/// The estimate of each version (`Decider::estimates`).
fn estimates(lm: &mut dyn LanguageModel, context: &str, options: &[&str]) -> Option<Vec<f64>> {
    emdysi_lm::decide::Decider::new(lm)
        .estimates(context, options)
        .ok()
}
