//! Show the readings of sentences compactly, for choosing the right one by
//! hand (the AI-prose treebank, `corpora/ai-prose/treebank.tsv`):
//! `cargo run --release -p emdysi-parse --example judge -- [K] < sentences.txt`.
//!
//! For each sentence: the tree of the best reading, then for each of the
//! next K-1 readings (default 6) only how it differs from the best: phrases
//! it groups that the best does not (`+[...]`), phrases of the best it does
//! not have (`-[...]`), and words analysed with a different lexical entry.
//! With `SKELETONS=1`, also each reading's derivation skeleton, the form the
//! treebank stores.

use std::collections::HashSet;
use std::time::Duration;

use emdysi_parse::*;

#[path = "common/tsdb.rs"]
mod tsdb;

fn spans(r: &Reading) -> HashSet<(usize, usize)> {
    let words: Vec<(usize, usize)> = r
        .nodes
        .iter()
        .filter(|n| n.leaf)
        .map(|n| (n.from, n.to))
        .collect();
    r.nodes
        .iter()
        .filter(|n| !n.leaf)
        .filter(|n| {
            words
                .iter()
                .filter(|&&(f, t)| f >= n.from && t <= n.to)
                .count()
                >= 2
        })
        .map(|n| (n.from, n.to))
        .collect()
}

fn bracket(s: &[char], (from, to): (usize, usize)) -> String {
    let mut to = to.min(s.len());
    while to > from && (s[to - 1].is_whitespace() || ",;:.!?".contains(s[to - 1])) {
        to -= 1;
    }
    format!("[{}]", s[from..to].iter().collect::<String>())
}

/// The skeleton of a derivation, as the trainer matches it.
pub fn skeleton_of(derivation: &str) -> String {
    tsdb::parse_sexp(derivation)
        .map(|t| tsdb::skeleton(&t, false))
        .unwrap_or_default()
}

fn main() {
    let k: usize = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(6);
    let mut erg = Erg::load(&default_grammar_dir()).unwrap();
    erg.config.timeout = Duration::from_secs(30);
    erg.config.max_readings = 100;
    erg.trees_for = 1;
    // CELL_BEAM=n (0: none) and FIRST_BEAM=n|none override chart pruning;
    // TIMEOUT=seconds the time limit.
    let env = |k: &str| std::env::var(k).ok();
    if let Some(b) = env("CELL_BEAM").and_then(|b| b.parse::<usize>().ok()) {
        erg.config.cell_beam = (b > 0).then_some(b);
    }
    if let Some(b) = env("FIRST_BEAM") {
        erg.first_beam = b.parse().ok();
    }
    if let Some(t) = env("TIMEOUT").and_then(|t| t.parse().ok()) {
        erg.config.timeout = Duration::from_secs(t);
    }
    let show_skeletons = std::env::var("SKELETONS").is_ok();
    for (n, line) in std::io::stdin().lines().map_while(Result::ok).enumerate() {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }
        let Ok(p) = erg.parse(&line) else {
            println!("## {n} [error] {line}");
            continue;
        };
        println!("## {n} [{} readings] {line}", p.readings.len());
        let Some(best) = p.readings.first() else {
            continue;
        };
        let chars: Vec<char> = line.chars().collect();
        if let Some(t) = &best.tree {
            println!("0 {} {}", best.root, t.bracketed());
        }
        if show_skeletons {
            println!("  {}", skeleton_of(&best.derivation));
        }
        let s0 = spans(best);
        for (i, r) in p
            .readings
            .iter()
            .enumerate()
            .skip(1)
            .take(k.saturating_sub(1))
        {
            let si = spans(r);
            let mut parts: Vec<String> = Vec::new();
            let mut plus: Vec<&(usize, usize)> = si.difference(&s0).collect();
            plus.sort();
            let mut minus: Vec<&(usize, usize)> = s0.difference(&si).collect();
            minus.sort();
            parts.extend(plus.iter().map(|&&sp| format!("+{}", bracket(&chars, sp))));
            parts.extend(minus.iter().map(|&&sp| format!("-{}", bracket(&chars, sp))));
            for (a, b) in best.words.iter().zip(&r.words) {
                if a.entry != b.entry && a.from == b.from {
                    parts.push(format!("{}: {}>{}", a.surface, a.entry, b.entry));
                }
            }
            if best.root != r.root {
                parts.push(format!("root {}", r.root));
            }
            if parts.is_empty() {
                parts.push("(same phrases and words; differs in rules)".into());
            }
            println!("{i} {}", parts.join("  "));
            if show_skeletons {
                println!("  {}", skeleton_of(&r.derivation));
            }
        }
    }
}
