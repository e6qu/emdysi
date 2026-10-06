//! Why sentences get no full analysis: for each sentence (one per line of
//! standard input) without a strict reading, each word in turn is replaced
//! by plain words of the same shape (*water*, *things*, *Kim*, *quickly*,
//! ...); the words whose replacement gives a strict reading are the
//! culprits. Counts over all sentences show which kinds of words the
//! grammar lacks most.
//!
//! `cargo run --release -p emdysi-parse --example culprits < sentences.txt`

use std::collections::BTreeMap;

use emdysi_parse::{Erg, default_grammar_dir, is_strict};

/// Plain words to stand in for `w`, tried in turn.
fn stand_ins(w: &str) -> &'static [&'static str] {
    let lower = w.to_lowercase();
    let capitalized = w.chars().next().is_some_and(char::is_uppercase)
        && w.chars().skip(1).any(char::is_lowercase);
    let all_caps = w.chars().all(|c| c.is_uppercase() || c.is_ascii_digit());
    if capitalized || all_caps {
        &["Kim"]
    } else if w.chars().any(|c| c.is_ascii_digit()) {
        &["ten"]
    } else if lower.ends_with("ly") {
        &["quickly"]
    } else if lower.ends_with('s') {
        &["things", "water"]
    } else {
        // A mass noun works without a determiner; a name, where a mass
        // noun does not.
        &["water", "Kim"]
    }
}

fn main() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let strict = |s: &str| {
        erg.parse(s)
            .map(|p| p.readings.iter().any(|r| is_strict(&r.root)))
            .unwrap_or(false)
    };
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let (mut total, mut failing, mut explained) = (0, 0, 0);
    for line in std::io::stdin().lines() {
        let line = line.unwrap();
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        total += 1;
        if strict(line) {
            continue;
        }
        failing += 1;
        let words: Vec<&str> = line.split(' ').collect();
        let mut found = Vec::new();
        for i in 0..words.len() {
            let w = words[i].trim_matches(|c: char| !c.is_alphanumeric());
            if w.is_empty() {
                continue;
            }
            for alt in stand_ins(w) {
                let replaced = words[i].replacen(w, alt, 1);
                let mut v = words.clone();
                v[i] = &replaced;
                if strict(&v.join(" ")) {
                    found.push(w.to_string());
                    break;
                }
            }
        }
        if !found.is_empty() {
            explained += 1;
            for w in &found {
                *counts.entry(w.clone()).or_default() += 1;
            }
        }
        println!("{}\t{}", found.join(", "), line);
    }
    let mut by: Vec<(String, usize)> = counts.into_iter().collect();
    by.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    eprintln!(
        "{total} sentences, {failing} without a strict reading, {explained} fixed by replacing one word"
    );
    for (w, n) in by.iter().take(40) {
        eprintln!("{n}\t{w}");
    }
}
