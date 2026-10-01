//! The bundled English word list (ESDB/SCOWL size 60; American and British
//! spellings, both -ise and -ize), used with
//! the grammar's lexicon for spelling.

use std::collections::HashMap;
use std::sync::OnceLock;

const WORDS: &str = include_str!("../../../data/scowl/en-60.tsv");

/// Word -> commonness tier (35 most common .. 60).
pub fn words() -> &'static HashMap<String, u8> {
    static MAP: OnceLock<HashMap<String, u8>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut m = HashMap::new();
        for line in WORDS.lines().filter(|l| !l.starts_with('#')) {
            if let Some((w, t)) = line.split_once('\t') {
                let tier = t.parse().unwrap_or(60);
                // Keep both the exact form and a lower-case key.
                m.insert(w.to_string(), tier);
                m.entry(w.to_lowercase()).or_insert(tier);
            }
        }
        m
    })
}

/// Commonness tier of a word form, if listed.
pub fn tier(word: &str) -> Option<u8> {
    let m = words();
    m.get(word).or_else(|| m.get(&word.to_lowercase())).copied()
}
