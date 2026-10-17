//! Where checking spends its time: each stage (parsing, re-parsing, the
//! grammar-error variant, the rules and the rules' own correction parses)
//! and the slowest rules, summed over the documents given as arguments
//! (Markdown, or plain text by extension), checked one after another with
//! the default packs and `THREADS` threads (default 1).
//!
//! `cargo run --release -p emdysi-check --example timing -- FILE...`
//!
//! Set `EMDYSI_NO_CACHE=1` to time parsing rather than the parse cache.
use emdysi_check::{Checker, Format, Options, Pack, analyze, timing};
use emdysi_parse::{Erg, default_grammar_dir};
use std::time::Instant;

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let packs: Vec<Pack> = [
        "core",
        "plain-style",
        "ai-tells",
        "structure",
        "terms",
        "substance",
    ]
    .iter()
    .filter_map(|n| std::fs::read_to_string(root.join(format!("packs/{n}.toml"))).ok())
    .map(|s| Pack::parse(&s).unwrap())
    .collect();
    let checker = Checker::new(packs);
    let mut erg = Erg::load(&default_grammar_dir()).unwrap();
    if let Some(dir) = emdysi_parse::parse_cache_dir() {
        erg.cache_parses(&dir);
    }
    let threads = std::env::var("THREADS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(1);
    let opts = Options {
        threads,
        ..Options::default()
    };
    let (mut sentences, mut words) = (0, 0);
    // Each parsed sentence's parse time, and whether it got a strict
    // reading and whether its search was complete.
    let mut parses: Vec<(f64, bool, bool)> = Vec::new();
    let (mut analyze_time, mut check_time) = (std::time::Duration::ZERO, std::time::Duration::ZERO);
    for path in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&path).unwrap();
        let format = if path.ends_with(".md") {
            Format::Markdown
        } else {
            Format::Plain
        };
        let t = Instant::now();
        let a = analyze(&erg, &src, format, &opts);
        analyze_time += t.elapsed();
        let t = Instant::now();
        checker.check(&erg, &a);
        check_time += t.elapsed();
        for p in a.sentences.iter().filter_map(|s| s.parse.as_ref()) {
            let strict = p.readings.iter().any(|r| emdysi_parse::is_strict(&r.root));
            parses.push((p.elapsed.as_secs_f64(), strict, p.complete));
        }
        sentences += a.sentences.len();
        words += a
            .sentences
            .iter()
            .map(|s| s.original.split_whitespace().count())
            .sum::<usize>();
    }
    let total: std::time::Duration = analyze_time + check_time;
    println!(
        "{sentences} sentences, {words} words; analyze {:.1}s, check {:.1}s (wall, {threads} threads); {:.0} ms a sentence",
        analyze_time.as_secs_f64(),
        check_time.as_secs_f64(),
        total.as_secs_f64() * 1000.0 / sentences.max(1) as f64
    );
    println!("\n| Stage | Seconds (all threads) | ms a sentence |\n|---|---|---|");
    for (name, d) in timing::report() {
        println!(
            "| {name} | {:.1} | {:.0} |",
            d.as_secs_f64(),
            d.as_secs_f64() * 1000.0 / sentences.max(1) as f64
        );
    }
    // Where the parse time goes: sentences by their parse time.
    let all: f64 = parses.iter().map(|p| p.0).sum();
    println!(
        "\n| Parse time | Sentences | Share of parse time | With a strict reading | Search complete |\n|---|---|---|---|---|"
    );
    for (lo, hi) in [
        (0.0, 0.5),
        (0.5, 1.0),
        (1.0, 2.0),
        (2.0, 5.0),
        (5.0, 9.5),
        (9.5, f64::MAX),
    ] {
        let b: Vec<&(f64, bool, bool)> = parses.iter().filter(|p| p.0 >= lo && p.0 < hi).collect();
        let t: f64 = b.iter().map(|p| p.0).sum();
        let label = if hi == f64::MAX {
            format!("{lo} s and over")
        } else {
            format!("{lo}-{hi} s")
        };
        println!(
            "| {label} | {} | {:.0}% | {} | {} |",
            b.len(),
            100.0 * t / all.max(1e-9),
            b.iter().filter(|p| p.1).count(),
            b.iter().filter(|p| p.2).count()
        );
    }
    let mut rules: Vec<(String, std::time::Duration)> = timing::RULE_TIMES
        .lock()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    rules.sort_by_key(|r| std::cmp::Reverse(r.1));
    println!("\n| Slowest rules | Seconds |\n|---|---|");
    for (id, d) in rules.iter().take(12) {
        println!("| `{id}` | {:.2} |", d.as_secs_f64());
    }
}
