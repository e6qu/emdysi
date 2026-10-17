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
