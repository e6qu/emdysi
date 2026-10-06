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
    // Closed-class words have their own tags whatever their case: a
    // sentence-initial |He| is a pronoun, not a candidate name (the ERG
    // makes a sentence-initial capitalized word a name when it is tagged
    // as a noun).
    if let Some(t) = closed_class(&lower) {
        return tags(t);
    }
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

/// Tags of closed-class words (Penn Treebank), as a statistical tagger
/// would give them.
fn closed_class(w: &str) -> Option<&'static [(&'static str, f64)]> {
    Some(match w {
        "i" | "you" | "he" | "she" | "it" | "we" | "they" | "me" | "him" | "us" | "them"
        | "myself" | "yourself" | "himself" | "herself" | "itself" | "ourselves" | "yourselves"
        | "themselves" => &[("PRP", 1.0)],
        "her" => &[("PRP", 0.5), ("PRP$", 0.5)],
        "my" | "your" | "his" | "its" | "our" | "their" => &[("PRP$", 1.0)],
        "the" | "a" | "an" | "these" | "those" | "every" | "each" | "another" | "no" => {
            &[("DT", 1.0)]
        }
        "this" | "some" | "any" | "all" | "both" | "either" | "neither" => &[("DT", 1.0)],
        "that" => &[("DT", 0.4), ("IN", 0.4), ("WDT", 0.2)],
        "to" => &[("TO", 1.0)],
        "in" | "on" | "at" | "of" | "for" | "with" | "from" | "by" | "about" | "into" | "onto"
        | "over" | "under" | "after" | "before" | "during" | "through" | "between" | "among"
        | "against" | "without" | "within" | "since" | "until" | "because" | "although"
        | "though" | "while" | "if" | "whether" | "as" | "than" | "upon" | "across" | "behind"
        | "beyond" | "toward" | "towards" | "despite" | "unless" => &[("IN", 1.0)],
        "and" | "or" | "but" | "nor" => &[("CC", 1.0)],
        "can" | "could" | "will" | "would" | "shall" | "should" | "may" | "might" | "must" => {
            &[("MD", 1.0)]
        }
        "is" | "has" | "does" => &[("VBZ", 1.0)],
        "are" | "have" | "do" => &[("VBP", 1.0)],
        "was" | "were" | "had" | "did" => &[("VBD", 1.0)],
        "be" => &[("VB", 1.0)],
        "been" => &[("VBN", 1.0)],
        "being" => &[("VBG", 1.0)],
        "which" => &[("WDT", 1.0)],
        "what" | "who" | "whom" => &[("WP", 1.0)],
        "whose" => &[("WP$", 1.0)],
        "when" | "where" | "why" | "how" => &[("WRB", 1.0)],
        "there" => &[("EX", 0.6), ("RB", 0.4)],
        "not" | "never" => &[("RB", 1.0)],
        _ => return None,
    })
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

    #[test]
    fn closed_class_words() {
        assert_eq!(tag("He", true)[0].tag, "PRP");
        assert_eq!(tag("The", true)[0].tag, "DT");
        assert!(
            tag("He", true)
                .iter()
                .all(|t| t.tag != "NN" && t.tag != "NNP")
        );
    }
}
