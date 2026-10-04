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

const RARE_WORDS: &str = include_str!("../../../data/scowl/en-70-extra.txt");
const TECH_WORDS: &[&str] = &[
    include_str!("../../../data/cspell/software-terms.txt"),
    include_str!("../../../data/cspell/software-tools.txt"),
    include_str!("../../../data/cspell/coding-terms.txt"),
    include_str!("../../../data/cspell/rust.txt"),
    include_str!("../../../data/cspell/k8s.txt"),
    include_str!("../../../data/cspell/fullstack.txt"),
];

/// Correct words outside the main list, accepted by the spelling check but
/// never suggested: rarer English words (ESDB size 70) and software terms
/// (cspell dictionaries). Lower-case keys.
fn accepted_words() -> &'static std::collections::HashSet<String> {
    static SET: OnceLock<std::collections::HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        RARE_WORDS
            .lines()
            .chain(TECH_WORDS.iter().flat_map(|t| t.lines()))
            .map(|l| l.split('#').next().unwrap_or("").trim())
            .filter(|w| {
                !w.is_empty()
                    && w.chars()
                        .all(|c| c.is_alphanumeric() || matches!(c, '\'' | '-' | '.'))
            })
            .map(str::to_lowercase)
            .collect()
    })
}

/// Whether a word is correct though not in the main list (see
/// [`accepted_words`]).
pub fn accepted(word: &str) -> bool {
    accepted_words().contains(&word.to_lowercase())
}

/// Commonness tier of a word form, if listed.
pub fn tier(word: &str) -> Option<u8> {
    let m = words();
    m.get(word).or_else(|| m.get(&word.to_lowercase())).copied()
}

const VARIANTS: &str = include_str!("../../../data/scowl/variants.tsv");

/// A spelling that belongs to one variety: American (`Us`) or British
/// (`Gb`), or British with `-ise` or `-ize` (Oxford) endings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Variety {
    Us,
    Gb,
    Ise,
    Ize,
}

impl Variety {
    pub fn parse(s: &str) -> Option<Variety> {
        match s {
            "us" => Some(Variety::Us),
            "gb" => Some(Variety::Gb),
            "ise" => Some(Variety::Ise),
            "ize" => Some(Variety::Ize),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Variety::Us => "American",
            Variety::Gb => "British",
            Variety::Ise => "-ise",
            Variety::Ize => "-ize",
        }
    }
}

/// Lower-case word -> the varieties it is specific to, each with the
/// corresponding spelling of the other variety (from ESDB, see
/// `data/scowl/SOURCE.md`).
pub fn variants() -> &'static HashMap<String, Vec<(Variety, String)>> {
    static MAP: OnceLock<HashMap<String, Vec<(Variety, String)>>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut m: HashMap<String, Vec<(Variety, String)>> = HashMap::new();
        for line in VARIANTS.lines().filter(|l| !l.starts_with('#')) {
            let mut f = line.split('\t');
            if let (Some(w), Some(Some(v)), Some(o)) =
                (f.next(), f.next().map(Variety::parse), f.next())
            {
                m.entry(w.to_string()).or_default().push((v, o.to_string()));
            }
        }
        m
    })
}

const ERG_ERRORS: &str = include_str!("../../../data/erg-errors/errors.tsv");

/// An entry of the ERG's grammar-error table: error class (`R` rule, `I`
/// incomplete, `D` determiner, `W` warning/awkward), feedback text (`$X`
/// stands for the word) and example sentences.
#[derive(Debug, Clone)]
pub struct ErgError {
    pub class: String,
    pub feedback: String,
    pub examples: Vec<String>,
}

/// Error code (an ERG rule, lexical entry, lexical type or root) -> entry.
pub fn erg_errors() -> &'static HashMap<String, ErgError> {
    static MAP: OnceLock<HashMap<String, ErgError>> = OnceLock::new();
    MAP.get_or_init(|| {
        ERG_ERRORS
            .lines()
            .filter(|l| !l.starts_with('#'))
            .filter_map(|l| {
                let mut f = l.split('\t');
                let code = f.next()?.to_string();
                let class = f.next()?.to_string();
                let feedback = f.next()?.to_string();
                let examples = f
                    .next()
                    .unwrap_or("")
                    .split(" | ")
                    .filter(|x| !x.is_empty())
                    .map(String::from)
                    .collect();
                Some((
                    code,
                    ErgError {
                        class,
                        feedback,
                        examples,
                    },
                ))
            })
            .collect()
    })
}
