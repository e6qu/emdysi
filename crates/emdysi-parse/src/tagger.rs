//! Rule-based part-of-speech hypotheses (decision D3).
//!
//! The ERG only uses tags to build generic lexical entries for words it does
//! not know; native entries win over generic ones in lexical filtering. So
//! the tagger proposes a small set of plausible Penn Treebank tags from the
//! word's shape and suffix rather than committing to one.

use crate::Tag;

fn tags(list: &[(&str, f64)]) -> Vec<Tag> {
    list.iter()
        .map(|&(t, p)| Tag {
            tag: t.to_string(),
            prob: p,
        })
        .collect()
}

/// Tag hypotheses for a token. `initial` is true for the first token of a
/// sentence, where capitalization says little.
pub fn tag(form: &str, initial: bool) -> Vec<Tag> {
    let chars: Vec<char> = form.chars().collect();
    if chars.is_empty() {
        return tags(&[("NN", 1.0)]);
    }
    if chars.iter().all(|c| !c.is_alphanumeric()) {
        let t = match form {
            "." | "?" | "!" => ".",
            "," => ",",
            ":" | ";" | "…" | "-" | "–" | "—" => ":",
            "(" | "[" | "{" => "(",
            ")" | "]" | "}" => ")",
            "“" | "‘" | "``" => "``",
            "”" | "’" | "''" => "''",
            "$" | "£" | "€" => "$",
            "#" => "#",
            _ => "SYM",
        };
        return tags(&[(t, 1.0)]);
    }
    if chars
        .iter()
        .all(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '/' | ':'))
    {
        return tags(&[("CD", 1.0)]);
    }
    let lower = form.to_lowercase();
    let capitalized = chars[0].is_uppercase();
    if capitalized && !initial {
        return tags(&[("NNP", 0.8), ("NN", 0.2)]);
    }
    let mut out = if lower.ends_with("ly") {
        vec![("RB", 0.6), ("JJ", 0.4)]
    } else if lower.ends_with("ing") {
        vec![("VBG", 0.6), ("NN", 0.3), ("JJ", 0.1)]
    } else if lower.ends_with("ed")
        && [
            "multi", "well-", "ill-", "self-", "full-", "high-", "low-", "long-", "short-",
        ]
        .iter()
        .any(|p| lower.starts_with(p))
    {
        // Compound adjectives in -ed: multifaceted, well-designed.
        vec![("JJ", 0.6), ("VBN", 0.4)]
    } else if lower.ends_with("ed") {
        vec![("VBD", 0.4), ("VBN", 0.4), ("JJ", 0.2)]
    } else if lower.ends_with("ous") {
        // Before the -us nouns (bonus, virus): meticulous, multifarious.
        vec![("JJ", 0.8), ("NN", 0.2)]
    } else if lower.ends_with("ss") || lower.ends_with("us") || lower.ends_with("is") {
        vec![("NN", 0.8), ("JJ", 0.2)]
    } else if lower.ends_with('s') {
        vec![("NNS", 0.6), ("VBZ", 0.4)]
    } else if [
        "able", "ible", "al", "ful", "ic", "ive", "less", "ous", "ish",
    ]
    .iter()
    .any(|s| lower.ends_with(s))
    {
        // The ERG drops a JJ hypothesis when NN has 0.3 or more.
        vec![("JJ", 0.8), ("NN", 0.2)]
    } else if ["ize", "ise", "ify", "ate"]
        .iter()
        .any(|s| lower.ends_with(s))
    {
        vec![("VB", 0.5), ("VBP", 0.3), ("NN", 0.2)]
    } else {
        vec![("NN", 0.6), ("JJ", 0.2), ("VB", 0.2)]
    };
    if capitalized && initial {
        out.push(("NNP", 0.3));
    }
    tags(&out)
}

#[cfg(test)]
mod tests {
    use super::tag;

    fn best(form: &str) -> String {
        tag(form, false)
            .into_iter()
            .max_by(|a, b| a.prob.total_cmp(&b.prob))
            .map(|t| t.tag)
            .unwrap_or_default()
    }

    #[test]
    fn suffixes() {
        assert_eq!(best("meticulous"), "JJ");
        assert_eq!(best("bonus"), "NN");
        assert_eq!(best("multifaceted"), "JJ");
        assert_eq!(best("well-designed"), "JJ");
        assert!(matches!(best("refactored").as_str(), "VBD" | "VBN"));
    }
}
