//! The grammar's judgments on minimal pairs it handles well (agreement,
//! irregular verb forms): the acceptable sentence gets a strict analysis
//! and the unacceptable one does not. See `examples/minimal_pairs.rs` for
//! the full evaluation.

use std::time::Duration;

use emdysi_parse::{Erg, default_grammar_dir, is_strict};

fn pairs(file: &str, paradigm: &str, n: usize) -> Vec<(String, String)> {
    let path = default_grammar_dir().join("../../corpora").join(file);
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split('\t').collect::<Vec<_>>())
        .filter(|f| f.len() >= 5 && f[0] == paradigm)
        .take(n)
        .map(|f| (f[3].to_string(), f[4].to_string()))
        .collect()
}

#[test]
fn agreement_and_verb_forms() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let accepts = |s: &str| {
        erg.parse_limited(s, Duration::from_secs(20), 20)
            .is_ok_and(|p| p.readings.iter().any(|r| is_strict(&r.root)))
    };
    let mut wrong = Vec::new();
    for (file, paradigm) in [
        (
            "blimp/sample.tsv",
            "regular_plural_subject_verb_agreement_2",
        ),
        ("blimp/sample.tsv", "irregular_past_participle_verbs"),
        (
            "zorro/sample.tsv",
            "agreement_subject_verb-across_relative_clause",
        ),
        (
            "zorro/sample.tsv",
            "agreement_determiner_noun-between_neighbors",
        ),
    ] {
        let ps = pairs(file, paradigm, 8);
        assert_eq!(ps.len(), 8, "{paradigm}");
        for (good, bad) in ps {
            if !accepts(&good) || accepts(&bad) {
                wrong.push(format!("{paradigm}: {good} / {bad}"));
            }
        }
    }
    // Upstream labels are not always right for the grammar (e.g. a bad
    // sentence with another reading); allow one miss.
    assert!(wrong.len() <= 1, "{wrong:#?}");
}
