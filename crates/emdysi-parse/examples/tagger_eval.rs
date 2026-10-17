//! The tagger against the gold Penn Treebank tags of UD English EWT
//! (corpora/ud-ewt): how often the gold tag is the tagger's first guess,
//! how often it is among its guesses, and how many guesses it makes, for
//! all words and for the words the ERG's lexicon does not know (the only
//! ones whose tags the grammar uses, for generic entries).
//!
//! `cargo run --release -p emdysi-parse --example tagger_eval [FILE.conllu]`
//! (default: the EWT dev set). Prints a Markdown table and the commonest
//! misses among unknown words (with `SHOW=1`, each miss on stderr). Only
//! words count: punctuation has rules of its own in the grammar.

use std::collections::HashMap;

use emdysi_parse::{Erg, default_grammar_dir, tagger};

#[derive(Default)]
struct Count {
    words: usize,
    first: usize,
    among: usize,
    guesses: usize,
}

impl Count {
    fn row(&self, name: &str) -> String {
        let pct = |n: usize| 100.0 * n as f64 / self.words.max(1) as f64;
        format!(
            "| {name} | {} | {:.1}% | {:.1}% | {:.2} |",
            self.words,
            pct(self.first),
            pct(self.among),
            self.guesses as f64 / self.words.max(1) as f64
        )
    }
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../corpora/ud-ewt/en_ewt-ud-dev.conllu"
        )
        .into()
    });
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    let (mut all, mut unknown) = (Count::default(), Count::default());
    let mut misses: HashMap<(String, String), usize> = HashMap::new();
    let mut initial = true;
    for line in text.lines() {
        if line.is_empty() {
            initial = true;
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        // Multiword tokens (|1-2|) and empty nodes (|8.1|) have no tag of
        // their own.
        if f.len() < 5 || f[0].contains(['-', '.']) {
            continue;
        }
        let (form, gold) = (f[1], f[4]);
        let tags = tagger::tag(form, initial);
        initial = false;
        // Punctuation has rules of its own in the ERG, not generic entries.
        if !form.chars().any(char::is_alphanumeric) {
            continue;
        }
        let known = erg.known_word(form);
        for c in [Some(&mut all), (!known).then_some(&mut unknown)]
            .into_iter()
            .flatten()
        {
            c.words += 1;
            c.guesses += tags.len();
            c.first += usize::from(tags.first().is_some_and(|t| t.tag == gold));
            c.among += usize::from(tags.iter().any(|t| t.tag == gold));
        }
        if !known && !tags.iter().any(|t| t.tag == gold) {
            let guessed = tags
                .iter()
                .map(|t| t.tag.as_str())
                .collect::<Vec<_>>()
                .join("/");
            *misses.entry((gold.to_string(), guessed)).or_default() += 1;
            if std::env::var_os("SHOW").is_some() {
                eprintln!("{gold}\t{form}");
            }
        }
    }
    println!("| Words | Count | First guess | Among guesses | Guesses a word |");
    println!("|---|---|---|---|---|");
    println!("{}", all.row("all"));
    println!("{}", unknown.row("unknown to the ERG"));
    let mut misses: Vec<_> = misses.into_iter().collect();
    misses.sort_by_key(|m| std::cmp::Reverse(m.1));
    println!("\n| Gold | Guessed | Unknown words |\n|---|---|---|");
    for ((gold, guessed), n) in misses.iter().take(15) {
        println!("| {gold} | {guessed} | {n} |");
    }
}
