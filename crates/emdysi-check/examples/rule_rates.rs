//! How often each rule fires on the texts of a corpus sample, per 1,000
//! words, for one or more text columns: e.g. machine output against its
//! expert edit in `corpora/beemo/sample.tsv`.
//!
//! `cargo run --release -p emdysi-check --example rule_rates -- corpora/beemo/sample.tsv 3,4 60`
//!
//! Arguments: the TSV file (a `#` header line; cells escaped as `\\`, `\t`,
//! `\n`, `\r`), the 0-based text columns (comma-separated), the number of
//! rows to use (default: all), and the packs to use (names in `packs/`
//! or paths to `.toml` files, comma-separated; default: the six default
//! packs). Texts are read as
//! Markdown. Prints a Markdown table.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use emdysi_check::*;
use emdysi_parse::{Erg, default_grammar_dir};

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(o) => out.push(o),
            None => out.push('\\'),
        }
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args.first().expect("TSV file");
    let cols: Vec<usize> = args
        .get(1)
        .expect("text columns, e.g. 3,4")
        .split(',')
        .map(|c| c.parse().expect("column number"))
        .collect();
    let limit: usize = args.get(2).map_or(usize::MAX, |n| n.parse().unwrap());
    let src = std::fs::read_to_string(path).unwrap();
    let header: Vec<String> = src
        .lines()
        .next()
        .unwrap_or("")
        .trim_start_matches("# ")
        .split(';')
        .next()
        .unwrap_or("")
        .split('\t')
        .map(String::from)
        .collect();
    let rows: Vec<Vec<String>> = src
        .lines()
        .filter(|l| !l.starts_with('#'))
        .take(limit)
        .map(|l| l.split('\t').map(unescape).collect())
        .collect();

    let packs_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs");
    let default = "core,ai-tells,plain-style,substance,structure,terms".to_string();
    let names: Vec<String> = args
        .get(3)
        .unwrap_or(&default)
        .split(',')
        .map(String::from)
        .collect();
    let packs: Vec<Pack> = names
        .iter()
        .map(|p| {
            Pack::parse(
                &std::fs::read_to_string(if p.ends_with(".toml") {
                    std::path::PathBuf::from(p)
                } else {
                    packs_dir.join(format!("{p}.toml"))
                })
                .unwrap(),
            )
            .unwrap()
        })
        .collect();
    let checker = Checker::new(packs);
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let opts = Options {
        threads: 1,
        timeout: Duration::from_secs(5),
        ..Options::default()
    };

    // (column, rule) -> count; column -> words
    let counts: Mutex<BTreeMap<(usize, String), usize>> = Mutex::new(BTreeMap::new());
    let words: Mutex<BTreeMap<usize, usize>> = Mutex::new(BTreeMap::new());
    let jobs: Vec<(usize, &str)> = rows
        .iter()
        .flat_map(|r| {
            cols.iter()
                .filter_map(|&c| r.get(c).map(|t| (c, t.as_str())))
        })
        .collect();
    let next = Mutex::new(0usize);
    std::thread::scope(|s| {
        for _ in 0..std::thread::available_parallelism().map_or(2, |n| n.get()) {
            s.spawn(|| {
                loop {
                    let k = {
                        let mut n = next.lock().unwrap();
                        *n += 1;
                        *n - 1
                    };
                    let Some(&(c, text)) = jobs.get(k) else { break };
                    let a = analyze(&erg, text, Format::Markdown, &opts);
                    let n: usize = a.sentences.iter().map(|s| s.word_count()).sum();
                    *words.lock().unwrap().entry(c).or_default() += n;
                    let mut cs = counts.lock().unwrap();
                    for d in checker.check(&erg, &a) {
                        *cs.entry((c, d.rule)).or_default() += 1;
                    }
                }
            });
        }
    });

    let counts = counts.into_inner().unwrap();
    let words = words.into_inner().unwrap();
    let mut rules: Vec<&String> = counts.keys().map(|(_, r)| r).collect();
    rules.sort();
    rules.dedup();
    let name = |c: usize| {
        header
            .get(c)
            .cloned()
            .unwrap_or_else(|| format!("column {c}"))
    };
    print!("| Rule |");
    for &c in &cols {
        print!(" {} |", name(c));
    }
    println!();
    println!("|---|{}", "---|".repeat(cols.len()));
    print!("| words |");
    for &c in &cols {
        print!(" {} |", words.get(&c).copied().unwrap_or(0));
    }
    println!();
    for r in rules {
        print!("| `{r}` |");
        for &c in &cols {
            let n = counts.get(&(c, r.clone())).copied().unwrap_or(0);
            let per = n as f64 * 1000.0 / words.get(&c).copied().unwrap_or(1).max(1) as f64;
            print!(" {n} ({per:.1}) |");
        }
        println!();
    }
}
