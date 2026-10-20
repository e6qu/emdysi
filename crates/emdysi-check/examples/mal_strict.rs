//! Whether the grammar-error variant tells real errors in sentences the
//! grammar accepts from correct sentences: for each grammar pair of the
//! real-error corpus whose sentence before the fix has a strict analysis,
//! that sentence and the corrected one are parsed with the variant, and
//! the ranker's best reading that names an error (a mal-rule or robust
//! entry) is compared with its best reading that names none.
//!
//! `cargo run --release -p emdysi-check --example mal_strict`
//!
//! Reported: how many sentences of each side get a reading that names an
//! error, how often it is the best reading, the margin of the best such
//! reading over the best without one (its score less the other's), and
//! how well that margin tells the sides apart (AUC: 0.5 is chance).
//! `THREADS` parses that many pairs at once.

use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use emdysi_check::*;
use emdysi_parse::{Erg, default_grammar_dir};

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files: Vec<_> = std::fs::read_dir(root.join("corpora/real-errors"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "tsv"))
        .collect();
    files.sort();
    let unescape = |s: &str| s.replace("\\n", "\n").replace("\\t", "\t");
    let rows: Vec<Vec<String>> = files
        .iter()
        .flat_map(|p| {
            std::fs::read_to_string(p)
                .unwrap()
                .lines()
                .filter(|l| !l.starts_with('#') && !l.is_empty())
                .map(|l| l.split('\t').map(unescape).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        })
        .filter(|r| r.len() >= 6 && r[1].starts_with("grammar"))
        .collect();
    let mut erg = Erg::load(&default_grammar_dir()).unwrap();
    if let Some(dir) = emdysi_parse::parse_cache_dir() {
        erg.cache_parses(&dir);
    }
    let mal = erg.mal().expect("the grammar-error variant");
    let opts = Options {
        threads: 1,
        ..Options::default()
    };
    // Per pair: (margin before the fix, margin after), each `None` when the
    // sentence has no strict analysis or no reading at all.
    let out: Mutex<Vec<(String, Option<f64>, Option<f64>)>> = Mutex::default();
    let next = Mutex::new(0usize);
    let threads: usize = std::env::var("THREADS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(2);
    // The sentence of `text` that holds byte offset `at`, if it has a strict
    // analysis.
    let sentence_at = |text: &str, at: usize| -> Option<String> {
        let a = analyze(&erg, text, Format::Markdown, &opts);
        (0..a.sentences.len())
            .find(|&si| {
                let r = a.sentence_source(si);
                r.start <= at && at <= r.end
            })
            .filter(|&si| a.sentences[si].strict())
            .map(|si| a.sentences[si].original.trim().to_string())
    };
    // The best reading naming an error less the best naming none.
    let margin = |sentence: &str| -> Option<f64> {
        let p = mal
            .parse_limited(sentence, Duration::from_secs(10), 100)
            .ok()?;
        let error = |r: &emdysi_parse::Reading| {
            r.nodes
                .iter()
                .any(|n| !n.leaf && (n.name.contains("_mal") || n.name.contains("_rbst")))
        };
        let best = |e: bool| {
            p.readings
                .iter()
                .filter(|r| error(r) == e)
                .map(|r| r.score)
                .fold(f64::NEG_INFINITY, f64::max)
        };
        let (with, without) = (best(true), best(false));
        (with > f64::NEG_INFINITY || without > f64::NEG_INFINITY).then_some(with - without)
    };
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let i = {
                        let mut n = next.lock().unwrap();
                        *n += 1;
                        *n - 1
                    };
                    let Some(r) = rows.get(i) else { break };
                    let (before, after) = (&r[4], &r[5]);
                    let p = before
                        .bytes()
                        .zip(after.bytes())
                        .take_while(|(a, b)| a == b)
                        .count();
                    let Some(b) = sentence_at(before, p) else {
                        continue;
                    };
                    let a = sentence_at(after, p);
                    let (mb, ma) = (margin(&b), a.as_deref().and_then(|a| margin(a)));
                    out.lock().unwrap().push((b, mb, ma));
                    if i % 20 == 0 {
                        eprintln!("{i} of {} pairs", rows.len());
                    }
                }
            });
        }
    });
    let out = out.into_inner().unwrap();
    let wrong: Vec<f64> = out.iter().filter_map(|x| x.1).collect();
    let right: Vec<f64> = out.iter().filter_map(|x| x.2).collect();
    let report = |name: &str, v: &[f64]| {
        let with = v.iter().filter(|&&m| m > f64::NEG_INFINITY).count();
        let best = v.iter().filter(|&&m| m > 0.0).count();
        // (`with` counts the sentences with any reading that names one.)
        println!(
            "{name}: {} sentences; a reading names an error in {with}; it is the best in {best}",
            v.len()
        );
    };
    report("before the fix (strict)", &wrong);
    report("after the fix", &right);
    // Only readings that name an error: above any margin; only readings
    // that name none: below.
    let finite = |v: &[f64]| -> Vec<f64> {
        v.iter()
            .map(|&m| {
                if m.is_finite() {
                    m
                } else if m > 0.0 {
                    1e9
                } else {
                    -1e9
                }
            })
            .collect()
    };
    for (name, v) in [("before the fix", &wrong), ("after the fix", &right)] {
        let only_errors = v.iter().filter(|&&m| m == f64::INFINITY).count();
        let only_clean = v.iter().filter(|&&m| m == f64::NEG_INFINITY).count();
        println!(
            "{name}: every reading names an error in {only_errors}, none does in {only_clean}, both kinds in {}",
            v.len() - only_errors - only_clean
        );
    }
    let (w, r) = (finite(&wrong), finite(&right));
    let mut s = 0.0;
    for x in &w {
        for y in &r {
            s += if x > y {
                1.0
            } else if x == y {
                0.5
            } else {
                0.0
            };
        }
    }
    println!(
        "AUC of the margin, before against after: {:.3}",
        s / (w.len() * r.len()).max(1) as f64
    );
    for t in [-4.0, -2.0, -1.0, 0.0, 1.0, 2.0] {
        let share = |v: &[f64]| {
            100.0 * v.iter().filter(|&&m| m >= t).count() as f64 / v.len().max(1) as f64
        };
        println!(
            "  margin {t:>4.1} or more: {:.0}% before the fix, {:.0}% after",
            share(&w),
            share(&r)
        );
    }
}
