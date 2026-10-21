//! Penn Treebank tags for the words of a reading, derived from the ERG's
//! lexical types and inflectional rules. Rules written for taggers
//! (LanguageTool's, see `scripts/import-languagetool.py`) constrain tokens
//! by these tags; here they come from the parse instead of a tagger.

use emdysi_parse::Word;

/// The Penn Treebank tag of `w` in its reading, or `None` for words the
/// tag set has no good answer for (the grammar's miscellaneous types).
pub fn penn(w: &Word) -> Option<&'static str> {
    let t = w.le_type.as_str();
    let s = w.surface.to_lowercase();
    let rule = |r: &str| w.rules.iter().any(|x| x.starts_with(r));
    if s.chars().any(|c| c.is_ascii_digit())
        && s.chars()
            .all(|c| c.is_ascii_digit() || c == ',' || c == '.')
    {
        return Some("CD");
    }
    let class = t.split('_').next().unwrap_or("");
    Some(match class {
        "n" if t.contains("_pr-there") => "EX",
        "n" if t.contains("_pr-wh") || t.contains("_wh") => "WP",
        "n" if t.contains("_pr-") || t.contains("_pr_") => "PRP",
        "n" if t.contains("_pn") => {
            if rule("n_pl") {
                "NNPS"
            } else {
                "NNP"
            }
        }
        "n" if t.contains("crd") || t.contains("card") => "CD",
        "n" => {
            if rule("n_pl") {
                "NNS"
            } else {
                "NN"
            }
        }
        "v" => verb(t, &s, &rule),
        "aj" if t.contains("crd") || t.contains("card") => "CD",
        "aj" if t.contains("-cmp") => "JJR",
        "aj" if t.contains("-sup") => "JJS",
        "aj" => "JJ",
        "av" if t.contains("-cmp") => "RBR",
        "av" if t.contains("-sup") => "RBS",
        "av" if t.contains("wh") => "WRB",
        "av" => "RB",
        "cm" if t.contains("vp_to") => "TO",
        "cm" => "IN",
        "p" | "pp" if s == "to" => "TO",
        "p" | "pp" => "IN",
        "d" if t.contains("poss") => "PRP$",
        "d" if t.contains("wh") => "WDT",
        "d" => "DT",
        "c" => "CC",
        _ => return None,
    })
}

/// Every tag `w` may have: the ERG gives the base form and the present
/// tense other than third person singular one rule (|stay| in *will stay*
/// and *they stay*), so such a word is both VB and VBP.
pub fn penn_all(w: &Word) -> Vec<&'static str> {
    match penn(w) {
        Some("VBP") if w.rules.iter().any(|r| r.starts_with("v_n3s-bse")) => vec!["VBP", "VB"],
        Some(t) => vec![t],
        None => Vec::new(),
    }
}

fn verb(t: &str, s: &str, rule: &dyn Fn(&str) -> bool) -> &'static str {
    if t.contains("mdl") || t.contains("_will") || t.contains("_would") {
        return "MD";
    }
    // Forms of *be*, *have* and *do* have entries of their own.
    match s {
        "am" | "are" | "'m" | "'re" => return "VBP",
        "is" | "'s" => return "VBZ",
        "was" | "were" => return "VBD",
        "been" => return "VBN",
        "being" => return "VBG",
        "be" => return "VB",
        "has" | "does" => return "VBZ",
        "had" | "did" => return "VBD",
        _ => {}
    }
    if rule("v_3s-fin") {
        "VBZ"
    } else if rule("v_pst") {
        "VBD"
    } else if rule("v_psp") || rule("v_pas") {
        "VBN"
    } else if rule("v_prp") {
        "VBG"
    } else if rule("v_n3s-bse") {
        "VBP"
    } else {
        "VB"
    }
}

/// Chunk tags for the words of a reading, in the form LanguageTool's
/// English chunker gives them (an OpenNLP chunker with its own additions):
/// a base noun phrase's words are `B-NP-…` (the first), `I-NP-…` and
/// `E-NP-…` (the last; a one-word phrase is both `B-` and `E-`), with
/// `singular` or `plural` after its head; a verb group's `B-VP`, `I-VP`; a
/// preposition `B-PP`; an adverb outside a verb group `B-ADVP`, `I-ADVP`; a
/// predicative adjective `B-ADJP`, `I-ADJP`; a subordinating conjunction
/// `B-SBAR`; anything else `O`. Derived from the Penn tags of [`penn`],
/// so from the parse rather than a statistical chunker.
pub fn chunks(words: &[Word]) -> Vec<Vec<String>> {
    let tags: Vec<&str> = words.iter().map(|w| penn(w).unwrap_or("")).collect();
    let lower: Vec<String> = words.iter().map(|w| w.surface.to_lowercase()).collect();
    let n = words.len();
    let mut out: Vec<Vec<String>> = vec![Vec::new(); n];
    let noun = |t: &str| matches!(t, "NN" | "NNS" | "NNP" | "NNPS");
    let premod = |t: &str| matches!(t, "DT" | "PRP$" | "CD" | "JJ" | "JJR" | "JJS" | "WDT");
    let verbal = |t: &str| matches!(t, "MD" | "VB" | "VBD" | "VBG" | "VBN" | "VBP" | "VBZ");
    let mut i = 0;
    while i < n {
        let t = tags[i];
        // A pronoun, existential |there| or a number on its own.
        if matches!(t, "PRP" | "EX" | "WP") {
            let num = if matches!(
                lower[i].as_str(),
                "we" | "us" | "they" | "them" | "ourselves" | "themselves"
            ) {
                "plural"
            } else {
                "singular"
            };
            out[i] = vec![format!("B-NP-{num}"), format!("E-NP-{num}")];
            i += 1;
            continue;
        }
        // A base noun phrase: premodifiers (an adverb before an adjective
        // among them) and nouns, ending at its last noun.
        if premod(t) || noun(t) {
            let mut j = i;
            let mut last_noun = None;
            while j < n {
                let u = tags[j];
                let adverb_in =
                    u == "RB" && j + 1 < n && matches!(tags[j + 1], "JJ" | "JJR" | "JJS") && j > i;
                if noun(u) {
                    last_noun = Some(j);
                } else if !(premod(u) || adverb_in) || last_noun.is_some() {
                    break;
                }
                j += 1;
            }
            let end = match last_noun {
                Some(e) => e,
                // No noun: a determiner or number alone is a noun phrase
                // (|these|, |two|); adjectives alone are an adjective phrase.
                None if matches!(t, "DT" | "CD") && (i + 1 >= n || !premod(tags[i + 1])) => i,
                None => {
                    let mut j = i;
                    while j < n && matches!(tags[j], "JJ" | "JJR" | "JJS") {
                        out[j] = vec![if j == i { "B-ADJP" } else { "I-ADJP" }.into()];
                        j += 1;
                    }
                    if j == i {
                        out[i] = vec!["O".into()];
                        j += 1;
                    }
                    i = j;
                    continue;
                }
            };
            let head = tags[end];
            let num = if matches!(head, "NNS" | "NNPS")
                || (head == "DT" && matches!(lower[end].as_str(), "these" | "those"))
                || (head == "CD" && lower[end] != "one" && lower[end] != "1")
            {
                "plural"
            } else {
                "singular"
            };
            for (k, slot) in out.iter_mut().enumerate().take(end + 1).skip(i) {
                *slot = if k == i && k == end {
                    vec![format!("B-NP-{num}"), format!("E-NP-{num}")]
                } else if k == i {
                    vec![format!("B-NP-{num}")]
                } else if k == end {
                    vec![format!("E-NP-{num}")]
                } else {
                    vec![format!("I-NP-{num}")]
                };
            }
            i = end + 1;
            continue;
        }
        // A verb group: modals, auxiliaries and verbs, |to| before a verb,
        // and adverbs between them.
        if verbal(t) || (t == "TO" && i + 1 < n && verbal(tags[i + 1])) {
            let mut j = i;
            while j < n {
                let u = tags[j];
                let inside = verbal(u)
                    || (u == "TO" && j + 1 < n && verbal(tags[j + 1]))
                    || (u == "RB" && j > i && j + 1 < n && verbal(tags[j + 1]));
                if !inside {
                    break;
                }
                out[j] = vec![if j == i { "B-VP" } else { "I-VP" }.into()];
                j += 1;
            }
            i = j;
            continue;
        }
        out[i] = vec![match t {
            "IN" if matches!(
                lower[i].as_str(),
                "because"
                    | "although"
                    | "though"
                    | "if"
                    | "whether"
                    | "unless"
                    | "whereas"
                    | "while"
            ) =>
            {
                "B-SBAR".to_string()
            }
            "IN" | "TO" => "B-PP".to_string(),
            "RB" | "RBR" | "RBS" | "WRB" => {
                if i > 0 && out[i - 1].iter().any(|c| c.ends_with("ADVP")) {
                    "I-ADVP".to_string()
                } else {
                    "B-ADVP".to_string()
                }
            }
            _ => "O".to_string(),
        }];
        i += 1;
    }
    out
}
