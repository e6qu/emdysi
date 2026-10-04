//! Train the parse-ranking model on the vendored gold profiles:
//! `cargo run --release --example train -- <cache-file> [profile dirs...]`.
//!
//! A source `treebank:FILE` reads a hand-judged treebank instead
//! (`corpora/ai-treebank/treebank.tsv`: id, source, sentence, gold
//! skeleton or `-`). With `TREEBANK_FOLDS=k`, the treebank items are also
//! evaluated by k-fold cross-validation: each fold is scored by a model
//! trained on everything else, and by a model trained without any treebank
//! item.
//!
//! Parses every grammatical item that has a gold derivation, records the
//! features of all readings (cached in `<cache-file>`, reused if present),
//! reports held-out accuracy on a 10% split, then trains on everything and
//! writes `crates/emdysi-parse/data/rank.tsv`.

use std::io::{BufRead, Write};
use std::sync::Mutex;
use std::time::Duration;

use emdysi_parse::rank::{Example, Model, train_maxent};
use emdysi_parse::*;

#[path = "common/tsdb.rs"]
mod tsdb;
use tsdb::*;

const PROFILES: &[&str] = &["mrs", "csli", "esd", "control", "ccs", "sh-spec"];

struct Cached {
    key: String,
    gold: usize,
    readings: Vec<Vec<String>>,
}

fn load_cache(path: &str) -> Vec<Cached> {
    let Ok(f) = std::fs::File::open(path) else {
        return Vec::new();
    };
    let mut out: Vec<Cached> = Vec::new();
    for line in std::io::BufReader::new(f).lines().map_while(Result::ok) {
        if let Some(rest) = line.strip_prefix("#\t") {
            let mut p = rest.split('\t');
            let key = p.next().unwrap_or_default().to_string();
            let gold = p.next().and_then(|g| g.parse().ok()).unwrap_or(0);
            out.push(Cached {
                key,
                gold,
                readings: Vec::new(),
            });
        } else if let Some(c) = out.last_mut() {
            c.readings.push(line.split(' ').map(String::from).collect());
        }
    }
    out
}

/// The ranker's trainer: the averaged perceptron (which made the shipped
/// model), or with `RANKER=maxent` the log-linear model, with `L2` (default
/// 1) and `ITERATIONS` (default 300).
fn train(examples: &[Example], epochs: usize) -> Model {
    let env = |k: &str| std::env::var(k).ok().and_then(|v| v.parse().ok());
    if std::env::var("RANKER").as_deref() == Ok("maxent") {
        return train_maxent(
            examples,
            env("L2").unwrap_or(1.0),
            env("ITERATIONS").map_or(300, |i: f64| i as usize),
        );
    }
    emdysi_parse::rank::train(examples, epochs)
}

/// Items of a hand-judged treebank with a gold skeleton.
fn treebank_items(path: &str) -> Vec<Item> {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("{path}: {e}"))
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            let (id, text, gold) = (*f.first()?, *f.get(2)?, *f.get(3)?);
            (gold != "-").then(|| Item {
                id: id.to_string(),
                text: text.to_string(),
                wf: true,
                gold: Some(gold.to_string()),
                mrs: None,
            })
        })
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cache_path = args.get(1).expect("cache file").clone();
    let dirs: Vec<String> = if args.len() > 2 {
        args[2..].to_vec()
    } else {
        let root = default_grammar_dir().join("../../corpora/erg-gold");
        PROFILES
            .iter()
            .map(|p| root.join(p).display().to_string())
            .collect()
    };

    let mut cached = load_cache(&cache_path);
    // Parse whatever is not cached yet; the cache is appended to as items
    // are parsed, so an interrupted run can be resumed.
    if std::env::var("NO_PARSE").is_err() {
        let mut erg = Erg::load(&default_grammar_dir()).unwrap();
        erg.config.timeout = Duration::from_secs(20);
        erg.config.max_readings = 500;
        erg.config.max_nodes = 30_000_000;
        erg.trees_for = 0;
        let max_words: usize = std::env::var("MAX_WORDS")
            .ok()
            .and_then(|n| n.parse().ok())
            .unwrap_or(usize::MAX);
        let done: std::collections::HashSet<String> =
            cached.iter().map(|c| c.key.clone()).collect();
        let mut work: Vec<(String, Item)> = Vec::new();
        for d in &dirs {
            let name = std::path::Path::new(d)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let source = if let Some(path) = d.strip_prefix("treebank:") {
                treebank_items(path)
            } else {
                items(d)
            };
            let name = if d.starts_with("treebank:") {
                "ai-treebank".to_string()
            } else {
                name
            };
            for it in source {
                let key = format!("{name}:{}", it.id);
                // MAX_WORDS skips long items, which are slow and
                // memory-hungry (long inputs are parsed with chart pruning).
                if it.wf
                    && it.gold.is_some()
                    && !done.contains(&key)
                    && it.text.split_whitespace().count() <= max_words
                {
                    work.push((key, it));
                }
            }
        }
        eprintln!("parsing {} items", work.len());
        let cache_file = Mutex::new(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&cache_path)
                .unwrap(),
        );
        let out = Mutex::new(Vec::new());
        let next = Mutex::new(0usize);
        std::thread::scope(|s| {
            let threads = std::env::var("THREADS")
                .ok()
                .and_then(|n| n.parse().ok())
                .unwrap_or_else(|| std::thread::available_parallelism().map_or(2, |n| n.get()));
            for _ in 0..threads {
                s.spawn(|| {
                    loop {
                        let i = {
                            let mut n = next.lock().unwrap();
                            *n += 1;
                            *n - 1
                        };
                        let Some((key, it)) = work.get(i) else { break };
                        if i % 100 == 0 {
                            eprintln!("  {i}");
                        }
                        let Ok(p) = erg.parse(&it.text) else { continue };
                        let gold = it.gold.as_deref().unwrap();
                        let idx = p.readings.iter().position(|r| {
                            parse_sexp(&r.derivation).is_some_and(|t| skeleton(&t, false) == gold)
                        });
                        if let Some(g) = idx {
                            let c = Cached {
                                key: key.clone(),
                                gold: g,
                                readings: p.readings.into_iter().map(|r| r.features).collect(),
                            };
                            let mut text = format!("#\t{}\t{}\n", c.key, c.gold);
                            for r in &c.readings {
                                text.push_str(&r.join(" "));
                                text.push('\n');
                            }
                            cache_file
                                .lock()
                                .unwrap()
                                .write_all(text.as_bytes())
                                .unwrap();
                            out.lock().unwrap().push(c);
                        }
                    }
                });
            }
        });
        cached.extend(out.into_inner().unwrap());
    }
    cached.sort_by(|a, b| a.key.cmp(&b.key));
    eprintln!(
        "{} items with the gold reading among the readings",
        cached.len()
    );

    let to_example = |c: &Cached| Example {
        readings: c.readings.clone(),
        gold: c.gold,
    };
    let is_test = |i: usize| i % 10 == 0;
    let train_keys: Option<std::collections::HashSet<String>> = std::env::var("TRAIN_KEYS")
        .ok()
        .map(|p| load_cache(&p).into_iter().map(|c| c.key).collect());
    let train_set: Vec<Example> = cached
        .iter()
        .enumerate()
        .filter(|(i, _)| !is_test(*i))
        // TRAIN_KEYS=<cache file> trains only on the items of an earlier
        // cache, to compare training sets on the same test split.
        .filter(|(_, c)| train_keys.as_ref().is_none_or(|k| k.contains(&c.key)))
        .map(|(_, c)| to_example(c))
        .collect();
    let test_set: Vec<&Cached> = cached
        .iter()
        .enumerate()
        .filter(|(i, _)| is_test(*i))
        .map(|(_, c)| c)
        .collect();
    let model = train(&train_set, 10);
    let accuracy = |m: &Model| -> (usize, usize) {
        let mut hit = 0;
        for c in &test_set {
            let best = (0..c.readings.len())
                .max_by(|&a, &b| m.score(&c.readings[a]).total_cmp(&m.score(&c.readings[b])))
                .unwrap_or(0);
            if best == c.gold || c.readings[best] == c.readings[c.gold] {
                hit += 1;
            }
        }
        (hit, test_set.len())
    };
    let (hit, n) = accuracy(&model);
    let (base, _) = accuracy(&Model::default());
    // OLD_MODEL=<rank.tsv> also scores another model on the same split.
    if let Ok(p) = std::env::var("OLD_MODEL") {
        let (old, _) = accuracy(&Model::parse(&std::fs::read_to_string(p).unwrap()));
        println!(
            "held-out exact match of {}: {old}/{}",
            std::env::var("OLD_MODEL").unwrap(),
            test_set.len()
        );
    }
    let ambiguous = test_set.iter().filter(|c| c.readings.len() > 1).count();
    println!(
        "held-out exact match: {hit}/{n} ({:.1}%); unranked first reading: {base}/{n}; {ambiguous} test items ambiguous",
        100.0 * hit as f64 / n.max(1) as f64
    );

    if let Some(k) = std::env::var("TREEBANK_FOLDS")
        .ok()
        .and_then(|k| k.parse::<usize>().ok())
    {
        let tb: Vec<&Cached> = cached
            .iter()
            .filter(|c| c.key.starts_with("ai-treebank:"))
            .collect();
        let rest: Vec<Example> = cached
            .iter()
            .filter(|c| !c.key.starts_with("ai-treebank:"))
            .map(to_example)
            .collect();
        let best_is_gold = |m: &Model, c: &Cached| {
            let best = (0..c.readings.len())
                .max_by(|&a, &b| m.score(&c.readings[a]).total_cmp(&m.score(&c.readings[b])))
                .unwrap_or(0);
            best == c.gold || c.readings[best] == c.readings[c.gold]
        };
        let without = train(&rest, 10);
        let base: usize = tb.iter().filter(|c| best_is_gold(&without, c)).count();
        let mut with = 0;
        for f in 0..k {
            let mut set: Vec<Example> = cached
                .iter()
                .filter(|c| !c.key.starts_with("ai-treebank:"))
                .map(to_example)
                .collect();
            // TREEBANK_WEIGHT=n repeats each treebank item n times.
            let weight: usize = std::env::var("TREEBANK_WEIGHT")
                .ok()
                .and_then(|w| w.parse().ok())
                .unwrap_or(1);
            for _ in 0..weight {
                set.extend(
                    tb.iter()
                        .enumerate()
                        .filter(|(i, _)| i % k != f)
                        .map(|(_, c)| to_example(c)),
                );
            }
            let m = train(&set, 10);
            with += tb
                .iter()
                .enumerate()
                .filter(|(i, c)| i % k == f && best_is_gold(&m, c))
                .count();
        }
        println!(
            "treebank ({} items, {k}-fold): best reading right {base} without the treebank, {with} with it",
            tb.len()
        );
    }

    let all: Vec<Example> = cached.iter().map(to_example).collect();
    let full = train(&all, 10);
    let out = default_grammar_dir().join("../../crates/emdysi-parse/data/rank.tsv");
    std::fs::write(&out, full.to_tsv()).unwrap();
    println!("wrote {} features to {}", full.weights.len(), out.display());
}
