//! The indefinite article: *a* before a consonant sound, *an* before a
//! vowel sound. The sound comes from LanguageTool's lists of words that
//! take *a* or *an* against their spelling (`data/languagetool/en/det_a.txt`,
//! `det_an.txt`), then from the CMU Pronouncing Dictionary
//! (`data/cmudict`), then from the spelling where the spelling settles it.
//! Words whose first sound is open (*an historic* and *a historic*, a word
//! with two pronunciations that differ) are skipped.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::structure::Hit;
use crate::{Analysis, Sentence};

const CMUDICT: &str = include_str!("../../../data/cmudict/cmudict.dict");
const DET_A: &str = include_str!("../../../data/languagetool/en/det_a.txt");
const DET_AN: &str = include_str!("../../../data/languagetool/en/det_an.txt");

/// ARPAbet vowels (without the stress digit).
const VOWELS: &[&str] = &[
    "AA", "AE", "AH", "AO", "AW", "AY", "EH", "ER", "EY", "IH", "IY", "OW", "OY", "UH", "UW",
];

/// The first sound of each word of the CMU Pronouncing Dictionary: a vowel
/// (`Some(true)`), a consonant (`Some(false)`), or open (`None`): its
/// pronunciations differ, or it starts with an *h* before an unstressed
/// vowel, where both articles are written (*an historic*, *a hotel*).
fn pronounced() -> &'static HashMap<String, Option<bool>> {
    static MAP: OnceLock<HashMap<String, Option<bool>>> = OnceLock::new();
    MAP.get_or_init(|| {
        let mut out: HashMap<String, Option<bool>> = HashMap::new();
        for line in CMUDICT.lines() {
            let mut it = line.split_whitespace();
            let (Some(word), Some(first)) = (it.next(), it.next()) else {
                continue;
            };
            let word = word.split('(').next().unwrap_or(word).to_string();
            let phone = first.trim_end_matches(|c: char| c.is_ascii_digit());
            let sound = if VOWELS.contains(&phone) {
                Some(true)
            } else if phone == "HH" {
                // The vowel after the h: stressed (|a house|), or not, where
                // usage is divided (|an historic|).
                it.next().filter(|v| v.ends_with('1')).map(|_| false)
            } else {
                Some(false)
            };
            out.entry(word)
                .and_modify(|s| {
                    if *s != sound {
                        *s = None;
                    }
                })
                .or_insert(sound);
        }
        out
    })
}

/// LanguageTool's words that take *a* (`false`) or *an* (`true`) against
/// their spelling, lower case unless marked case-sensitive with `*`;
/// words in both lists take either and are left out.
fn listed() -> &'static HashMap<String, bool> {
    static MAP: OnceLock<HashMap<String, bool>> = OnceLock::new();
    MAP.get_or_init(|| {
        let read = |src: &str| -> HashSet<String> {
            src.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .map(|l| match l.strip_prefix('*') {
                    Some(exact) => exact.to_string(),
                    None => l.to_lowercase(),
                })
                .collect()
        };
        let (a, an) = (read(DET_A), read(DET_AN));
        let mut out = HashMap::new();
        for w in a.difference(&an) {
            out.insert(w.clone(), false);
        }
        for w in an.difference(&a) {
            out.insert(w.clone(), true);
        }
        out
    })
}

/// Whether `form` (as written) starts with a vowel sound, from the lists,
/// the dictionary or the spelling, if one of them settles it.
fn sound(form: &str) -> Option<bool> {
    let lower = form.to_lowercase();
    if let Some(&an) = listed().get(form).or_else(|| listed().get(&lower)) {
        return Some(an);
    }
    if lower == "one" || lower == "once" || lower.starts_with("one-") {
        return Some(false);
    }
    if let Some(&s) = pronounced().get(&lower) {
        return s;
    }
    vowel_sound(&lower)
}

/// Words spelled with a vowel letter that start with a consonant sound
/// (prefixes).
const CONSONANT_SOUND: &[&str] = &[
    "unique",
    "unit",
    "univers",
    "uniform",
    "unif",
    "union",
    "unicode",
    "unicorn",
    "unison",
    "unicast",
    "unilateral",
    "unidirectional",
    "unary",
    "use",
    "usa",
    "usu",
    "usur",
    "uter",
    "util",
    "utopi",
    "ura",
    "ure",
    "uri",
    "uro",
    "eu",
    "ewe",
    "ouija",
    "ukulele",
    "ubiq",
];

/// Words spelled with a consonant letter that start with a vowel sound
/// (prefixes).
const VOWEL_SOUND: &[&str] = &["hour", "honest", "honor", "honour", "heir"];

/// Function words: an *a* or *an* before one of these is not an article
/// ("options a and b").
const FUNCTION_WORDS: &[&str] = &[
    "about", "above", "across", "after", "again", "against", "all", "also", "am", "among",
    "amongst", "an", "and", "any", "are", "around", "as", "at", "be", "but", "by", "can", "did",
    "do", "does", "each", "eg", "either", "else", "etc", "even", "ever", "every", "for", "from",
    "had", "has", "have", "he", "her", "his", "ie", "if", "in", "into", "is", "it", "its",
    "itself", "may", "might", "must", "my", "no", "nor", "not", "of", "off", "often", "on", "once",
    "only", "onto", "or", "other", "our", "out", "over", "own", "she", "should", "so", "than",
    "that", "the", "their", "then", "there", "these", "they", "this", "those", "to", "too",
    "under", "unless", "until", "up", "upon", "us", "was", "we", "were", "what", "when", "where",
    "which", "who", "will", "with", "would", "you", "your",
];

/// Whether `word` (lower case) starts with a vowel sound, if the spelling
/// settles it.
fn vowel_sound(word: &str) -> Option<bool> {
    if word == "one" || word == "once" || word.starts_with("one-") {
        return Some(false);
    }
    if CONSONANT_SOUND.iter().any(|p| word.starts_with(p)) {
        return Some(false);
    }
    if VOWEL_SOUND.iter().any(|p| word.starts_with(p)) {
        return Some(true);
    }
    let first = word.chars().next()?;
    match first {
        'a' | 'e' | 'i' | 'o' => Some(true),
        // un- (but not uni-), up-, um-, ul-, ug-, ut-, ur-: a vowel sound.
        'u' => {
            let open = ["un", "up", "um", "ul", "ug", "ut", "ur"]
                .iter()
                .any(|p| word.starts_with(p));
            (open && !word.starts_with("uni")).then_some(true)
        }
        // h- is open (*a historic*, *an historic*); x- is /z/ or /eks/.
        'h' | 'x' => None,
        'b' | 'c' | 'd' | 'f' | 'g' | 'j' | 'k' | 'l' | 'm' | 'n' | 'p' | 'q' | 'r' | 's' | 't'
        | 'v' | 'w' | 'y' | 'z' => Some(false),
        _ => None,
    }
}

/// *a* before a vowel sound and *an* before a consonant sound, where the
/// article is a determiner in the best analysis and the next word is a
/// listed lower-case word.
pub fn run_articles(a: &Analysis) -> Vec<Hit> {
    let mut out = Vec::new();
    for (si, s) in a.sentences.iter().enumerate() {
        for (ti, pair) in s.tokens.windows(2).enumerate() {
            let (art, next) = (&pair[0], &pair[1]);
            let an = match art.form.as_str() {
                "a" => false,
                "an" => true,
                "A" if ti == 0 => false,
                "An" if ti == 0 => true,
                _ => continue,
            };
            if !gap_is_space(s, art.to, next.from) {
                continue;
            }
            // A hyphenated word the dictionary lists whole (|x-ray|), else
            // its first part.
            let whole = next.form.to_lowercase();
            let word = if next.form.contains('-') && pronounced().contains_key(&whole) {
                next.form.as_str()
            } else {
                next.form.split('-').next().unwrap_or("")
            };
            // An acronym in capitals counts where the lists or the
            // dictionary know how it is said (|an MBA|, |a NASA|).
            let acronym = (2..=6).contains(&word.chars().count())
                && word.chars().all(|c| c.is_ascii_uppercase())
                && (listed().contains_key(word)
                    || listed().contains_key(&word.to_lowercase())
                    || pronounced().contains_key(&word.to_lowercase()));
            if !acronym
                && (word.chars().count() < 2
                    || !word.chars().all(|c| c.is_ascii_lowercase() || c == '\'' || c == '-')
                    || !word.chars().any(|c| "aeiouy".contains(c))
                    || FUNCTION_WORDS.contains(&word)
                    || crate::dict::tier(word).is_none()
                    // A lower-case acronym ("an mri").
                    || crate::dict::words().contains_key(&word.to_uppercase()))
            {
                continue;
            }
            let Some(vowel) = sound(word) else {
                continue;
            };
            if vowel != an && is_determiner(s, art.from, art.to) {
                let fixed = match art.form.as_str() {
                    "a" => "an",
                    "an" => "a",
                    "A" => "An",
                    _ => "A",
                };
                let mut h = Hit::at(a, si, art.from, art.to);
                h = h.var("fix", fixed).var("word", &next.form);
                // Before a verb form (|Instead of an creating a block|)
                // the article itself may be the error: a suggestion, not
                // an automatic fix.
                let verb_next = s.best().is_some_and(|r| {
                    r.words
                        .iter()
                        .filter(|w| w.from < next.to && next.from < w.to)
                        .filter_map(crate::tags::penn)
                        .any(|t| t.starts_with("VB"))
                });
                if verb_next {
                    h.suggestions = vec![fixed.to_string()];
                } else {
                    h.replacement = Some(fixed.to_string());
                }
                out.push(h);
            }
        }
    }
    out
}

fn gap_is_space(s: &Sentence, from: usize, to: usize) -> bool {
    to > from
        && s.original
            .chars()
            .skip(from)
            .take(to - from)
            .all(|c| c == ' ')
}

fn is_determiner(s: &Sentence, from: usize, to: usize) -> bool {
    s.best().is_some_and(|r| {
        r.words
            .iter()
            .any(|w| w.from == from && w.to == to && w.le_type.starts_with("d_"))
    })
}

#[cfg(test)]
mod tests {
    use super::{sound, vowel_sound};

    #[test]
    fn sounds_from_the_lists_and_the_dictionary() {
        for w in [
            "hour",
            "honest",
            "unidentified",
            "x-ray",
            "MBA",
            "eight",
            "heir",
        ] {
            assert_eq!(sound(w), Some(true), "{w}");
        }
        for w in [
            "house",
            "unicorn",
            "euro",
            "one-time",
            "xenon",
            "university",
            "NASA",
        ] {
            assert_eq!(sound(w), Some(false), "{w}");
        }
        // Usage is divided before an unstressed h.
        for w in ["historic", "hotel"] {
            assert_eq!(sound(w), None, "{w}");
        }
    }

    #[test]
    fn sounds() {
        for w in [
            "overview",
            "alpha",
            "additional",
            "update",
            "unusual",
            "hour",
            "eight",
        ] {
            assert_eq!(vowel_sound(w), Some(true), "{w}");
        }
        for w in [
            "unique",
            "user",
            "one",
            "european",
            "cluster",
            "dependent",
            "usual",
        ] {
            assert_eq!(vowel_sound(w), Some(false), "{w}");
        }
        for w in ["unidentified", "historic", "xenon", "usb"] {
            assert_eq!(vowel_sound(w), None, "{w}");
        }
    }
}
