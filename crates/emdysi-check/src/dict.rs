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

/// Common words (up to tier [`RELATED_TIER`]), lower case and alphabetic, by
/// their first four letters.
pub fn by_prefix(prefix: &str) -> &'static [String] {
    static MAP: OnceLock<HashMap<String, Vec<String>>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut m: HashMap<String, Vec<String>> = HashMap::new();
        for (w, &t) in words() {
            if t <= RELATED_TIER
                && w.chars().count() >= 4
                && w.chars().all(|c| c.is_ascii_lowercase())
            {
                m.entry(w[..4].to_string()).or_default().push(w.clone());
            }
        }
        for v in m.values_mut() {
            v.sort();
        }
        m
    })
    .get(prefix)
    .map_or(&[], Vec::as_slice)
}

/// The commonest tier [`by_prefix`] lists.
const RELATED_TIER: u8 = 60;

const RARE_WORDS: &str = include_str!("../../../data/scowl/en-70-extra.txt");
const RARER_WORDS: &str = include_str!("../../../data/scowl/en-80-extra.txt");
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

/// Whether a word is accepted without the rarest list (`en-80-extra`):
/// for checks that a rare form would wrongly silence (|informations| is a
/// size-80 word, but a mass noun made plural).
pub fn accepted_common(word: &str) -> bool {
    let w = word.to_lowercase();
    accepted_words().contains(&w) || accepted(&w) && !rarest_words().contains(&w)
}

/// The words of ESDB size 80 beyond the other lists, lower case.
fn rarest_words() -> &'static std::collections::HashSet<String> {
    static SET: OnceLock<std::collections::HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        RARER_WORDS
            .lines()
            .filter(|l| !l.starts_with('#'))
            .map(|l| l.trim().to_lowercase())
            .filter(|w| !w.is_empty())
            .collect()
    })
}

/// The words of the technical lists, lower case.
fn tech_words() -> &'static std::collections::HashSet<String> {
    static SET: OnceLock<std::collections::HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        TECH_WORDS
            .iter()
            .flat_map(|t| t.lines())
            .map(|l| l.split('#').next().unwrap_or("").trim().to_lowercase())
            .filter(|w| !w.is_empty())
            .collect()
    })
}

/// Whether a word is a technical term (the cspell lists: |vSphere|,
/// |kubelet|) and not in the main list.
pub fn technical(word: &str) -> bool {
    let w = word.to_lowercase();
    tier(&w).is_none() && tech_words().contains(&w)
}

/// Whether a word is correct though not in the main list (see
/// [`accepted_words`]).
pub fn accepted(word: &str) -> bool {
    let w = word.to_lowercase();
    if accepted_words().contains(&w) || rarest_words().contains(&w) {
        return true;
    }
    // Only for the technical lists: the rare English words already list
    // their forms, and inferring more lets typos through (|diabled| as
    // |diable| plus |-d|).
    let set = tech_words();
    // A plural or verb form of an accepted word (|impls| from |impl|,
    // |rebased| from |rebase|), or the singular of an accepted plural
    // (|finalizer| from |finalizers|): the technical lists name one form.
    let stem = |suffix: &str| w.strip_suffix(suffix).filter(|s| s.chars().count() >= 3);
    // (Not a plural of a plural: |valuess| is no form of |values|.)
    stem("s").is_some_and(|s| !s.ends_with('s') && set.contains(s))
        || stem("es").is_some_and(|s| set.contains(s))
        || stem("ed").is_some_and(|s| set.contains(s) || set.contains(&format!("{s}e")))
        || stem("d").is_some_and(|s| s.ends_with('e') && set.contains(s))
        || stem("ing").is_some_and(|s| set.contains(s) || set.contains(&format!("{s}e")))
        || set.contains(&format!("{w}s"))
        // A verb whose agent noun is accepted (|autoscaled|, |autoscale|
        // from |autoscalers|).
        || ["ed", "es", "ing", "e", "s"].iter().any(|x| {
            stem(x).is_some_and(|s| {
                set.contains(&format!("{s}er")) || set.contains(&format!("{s}ers"))
            })
        })
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

const CODESPELL: &str = include_str!("../../../data/codespell/dictionary.txt");
const CODESPELL_RARE: &str = include_str!("../../../data/codespell/dictionary_rare.txt");

/// Codespell's `wrong->right, right` lines: each misspelling's
/// corrections, and whether the first may be applied without asking (a
/// single correction, not marked with a trailing comma).
fn codespell_map(text: &str) -> HashMap<String, (Vec<String>, bool)> {
    text.lines()
        .filter_map(|l| {
            let (wrong, right) = l.split_once("->")?;
            let fixes: Vec<String> = right
                .split(',')
                .map(str::trim)
                .filter(|x| !x.is_empty())
                .map(str::to_string)
                .collect();
            let sure = fixes.len() == 1 && !right.trim_end().ends_with(',');
            Some((wrong.trim().to_string(), (fixes, sure)))
        })
        .collect()
}

/// A misspelling from codespell's dictionary of common misspellings
/// (`data/codespell`, from English Wikipedia's lists): its corrections, and
/// whether the first may be applied without asking (a single correction,
/// not marked with a trailing comma).
pub fn codespell(word: &str) -> Option<(&'static [String], bool)> {
    static MAP: OnceLock<HashMap<String, (Vec<String>, bool)>> = OnceLock::new();
    MAP.get_or_init(|| codespell_map(CODESPELL))
        .get(&word.to_lowercase())
        .map(|(f, sure)| (f.as_slice(), *sure))
}

/// A real but rare word that codespell lists as usually a misspelling
/// (|manger| for |manager|, |wit| for |with|): its corrections.
pub fn codespell_rare(word: &str) -> Option<&'static [String]> {
    static MAP: OnceLock<HashMap<String, (Vec<String>, bool)>> = OnceLock::new();
    MAP.get_or_init(|| codespell_map(CODESPELL_RARE))
        .get(&word.to_lowercase())
        .map(|(f, _)| f.as_slice())
}

const UNIMORPH: &str = include_str!("../../../data/unimorph/eng");

/// Whether `word` is an inflected form, in UniMorph's English tables
/// (`data/unimorph`, from Wiktionary), of a lemma the word lists know
/// (|nightlies| of |nightly|). Only forms of known lemmas count: the
/// tables also list rare and obsolete words, which would hide typos.
pub fn inflection_of_known(word: &str) -> bool {
    static MAP: OnceLock<HashMap<String, Vec<String>>> = OnceLock::new();
    let map = MAP.get_or_init(|| {
        let mut m: HashMap<String, Vec<String>> = HashMap::new();
        for line in UNIMORPH.lines() {
            let mut it = line.split('\t');
            let (Some(lemma), Some(form)) = (it.next(), it.next()) else {
                continue;
            };
            if lemma.contains(' ') || form.contains(' ') || form == lemma {
                continue;
            }
            m.entry(form.to_lowercase())
                .or_default()
                .push(lemma.to_lowercase());
        }
        m
    });
    map.get(&word.to_lowercase())
        .is_some_and(|lemmas| lemmas.iter().any(|l| tier(l).is_some()))
}

const COHERENCY: &str = include_str!("../../../data/languagetool/en/coherency.txt");

/// The spelling one term is grouped under when a document writes it in two
/// ways both valid in one variety (|acknowledgement| and |acknowledgment|,
/// |randomiser| and |randomizer|): LanguageTool's coherency list
/// (`data/languagetool/en/coherency.txt`), with the inflected forms of
/// each pair from UniMorph matched by their features. `word` is lowercase
/// without hyphens. Pairs the variety tables already relate are left to
/// the spelling-consistency rule.
pub fn coherent_key(word: &str) -> Option<&'static str> {
    static MAP: OnceLock<HashMap<String, String>> = OnceLock::new();
    let map = MAP.get_or_init(|| {
        let plain = |w: &str| w.trim().to_lowercase().replace('-', "");
        let pairs: Vec<Vec<String>> = COHERENCY
            .lines()
            .filter(|l| !l.starts_with('#') && l.contains(';'))
            .map(|l| l.split(';').map(plain).collect())
            .filter(|p: &Vec<String>| {
                !p.iter().any(|a| {
                    variants()
                        .get(a)
                        .is_some_and(|vs| vs.iter().any(|(_, o)| p.contains(&o.to_lowercase())))
                })
            })
            .collect();
        let wanted: std::collections::HashSet<&str> =
            pairs.iter().flatten().map(String::as_str).collect();
        // Lemma -> (features, form), for the lemmas of the pairs.
        let mut forms: HashMap<&str, Vec<(&str, String)>> = HashMap::new();
        for line in UNIMORPH.lines() {
            let mut it = line.split('\t');
            if let (Some(lemma), Some(form), Some(feats)) = (it.next(), it.next(), it.next()) {
                if let Some(&l) = wanted.get(lemma.to_lowercase().as_str()) {
                    forms.entry(l).or_default().push((feats, plain(form)));
                }
            }
        }
        let mut m = HashMap::new();
        for p in &pairs {
            let first = &p[0];
            for other in p {
                m.insert(other.clone(), first.clone());
                for (feats, form) in forms.get(other.as_str()).into_iter().flatten() {
                    if let Some((_, f)) = forms
                        .get(first.as_str())
                        .and_then(|fs| fs.iter().find(|(g, _)| g == feats))
                    {
                        m.insert(form.clone(), f.clone());
                    }
                }
            }
        }
        m
    });
    map.get(word).map(String::as_str)
}

/// `word` inflected for a Penn Treebank tag (VBD, VBN, VBG, VBZ, VB, VBP,
/// NNS, NN), from UniMorph's English tables: the form with that tag of a
/// lemma `word` is a form of (|go| or |went| with VBN gives |gone|). `None`
/// when the tables do not have it. The case of the result is lower.
pub fn inflect(word: &str, tag: &str) -> Option<String> {
    type Forms = HashMap<String, Vec<(String, String)>>;
    static MAP: OnceLock<(Forms, HashMap<String, Vec<String>>)> = OnceLock::new();
    let (forms, lemmas) = MAP.get_or_init(|| {
        let mut forms: Forms = HashMap::new();
        let mut lemmas: HashMap<String, Vec<String>> = HashMap::new();
        for line in UNIMORPH.lines() {
            let mut it = line.split('\t');
            let (Some(l), Some(f), Some(feats)) = (it.next(), it.next(), it.next()) else {
                continue;
            };
            if l.contains(' ') || f.contains(' ') {
                continue;
            }
            let (l, f) = (l.to_lowercase(), f.to_lowercase());
            forms
                .entry(l.clone())
                .or_default()
                .push((feats.to_string(), f.clone()));
            let ls = lemmas.entry(f).or_default();
            if !ls.contains(&l) {
                ls.push(l);
            }
        }
        (forms, lemmas)
    });
    let w = word.to_lowercase();
    let want = match tag {
        "VBD" => "V;PST",
        "VBN" => "V;V.PTCP;PST",
        "VBG" => "V;V.PTCP;PRS",
        "VBZ" => "V;PRS;3;SG",
        "NNS" => "N;PL",
        "VB" | "VBP" | "NN" => "",
        _ => return None,
    };
    let noun = tag.starts_with('N');
    let candidates: Vec<&String> = std::iter::once(&w)
        .filter(|l| forms.contains_key(*l))
        .chain(lemmas.get(&w).into_iter().flatten())
        .collect();
    for l in candidates {
        let fs = &forms[l];
        // The lemma must have forms of the right word class.
        if !fs
            .iter()
            .any(|(f, _)| f.starts_with(if noun { "N" } else { "V" }))
        {
            continue;
        }
        if want.is_empty() {
            return Some(l.clone());
        }
        if let Some((_, f)) = fs.iter().find(|(f, _)| f == want) {
            return Some(f.clone());
        }
    }
    None
}

#[cfg(test)]
mod inflect_tests {
    use super::inflect;

    #[test]
    fn forms_for_tags() {
        assert_eq!(inflect("went", "VBN").as_deref(), Some("gone"));
        assert_eq!(inflect("go", "VBZ").as_deref(), Some("goes"));
        assert_eq!(inflect("mice", "NN").as_deref(), Some("mouse"));
        assert_eq!(inflect("mouse", "NNS").as_deref(), Some("mice"));
        assert_eq!(inflect("writes", "VB").as_deref(), Some("write"));
    }
}
