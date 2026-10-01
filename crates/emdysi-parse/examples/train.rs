//! Train the parse-ranking model on the vendored gold profiles:
//! `cargo run --release --example train -- <cache-file> [profile dirs...]`.
//!
//! Parses every grammatical item that has a gold derivation, records the
//! features of all readings (cached in `<cache-file>`, reused if present),
//! reports held-out accuracy on a 10% split, then trains on everything and
//! writes `crates/emdysi-parse/data/rank.tsv`.

use std::io::{BufRead, Write};
use std::sync::Mutex;
use std::time::Duration;

use emdysi_parse::rank::{Example, Model, train};
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
                .unwrap()
                .to_string_lossy()
                .to_string();
            for it in items(d) {
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
    let train_set: Vec<Example> = cached
        .iter()
        .enumerate()
        .filter(|(i, _)| !is_test(*i))
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
    let ambiguous = test_set.iter().filter(|c| c.readings.len() > 1).count();
    println!(
        "held-out exact match: {hit}/{n} ({:.1}%); unranked first reading: {base}/{n}; {ambiguous} test items ambiguous",
        100.0 * hit as f64 / n.max(1) as f64
    );

    let all: Vec<Example> = cached.iter().map(to_example).collect();
    let full = train(&all, 10);
    let out = default_grammar_dir().join("../../crates/emdysi-parse/data/rank.tsv");
    std::fs::write(&out, full.to_tsv()).unwrap();
    println!("wrote {} features to {}", full.weights.len(), out.display());
}
