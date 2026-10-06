//! The indefinite article: *a* before a consonant sound, *an* before a
//! vowel sound. The sound is read from the spelling only where the spelling
//! settles it; words whose first sound the spelling leaves open (*uni-*,
//! *h-*, *x-*, acronyms, single letters, numbers) are skipped.

use crate::structure::Hit;
use crate::{Analysis, Sentence};

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
            let word = next.form.split('-').next().unwrap_or("");
            if word.chars().count() < 2
                || !word.chars().all(|c| c.is_ascii_lowercase() || c == '\'')
                || !word.chars().any(|c| "aeiouy".contains(c))
                || FUNCTION_WORDS.contains(&word)
                || crate::dict::tier(word).is_none()
                // A lower-case acronym ("an mri").
                || crate::dict::words().contains_key(&word.to_uppercase())
            {
                continue;
            }
            let Some(vowel) = vowel_sound(word) else {
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
                h.replacement = Some(fixed.to_string());
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
    use super::vowel_sound;

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
