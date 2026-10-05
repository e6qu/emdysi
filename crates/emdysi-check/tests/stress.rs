//! Grammatical stress sentences (`corpora/stress/stress.tsv`: the buffalo
//! sentence, *had had*, garden paths, center embedding, ambiguity): the
//! `core` pack, whose rules claim errors, must claim none in any of them;
//! nearly all get a full analysis; and the famous ambiguous ones are
//! reported as ambiguous.

use std::path::Path;
use std::time::Duration;

use emdysi_check::*;
use emdysi_parse::{Erg, default_grammar_dir};

#[test]
fn stress_sentences() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let rows: Vec<(String, String)> =
        std::fs::read_to_string(root.join("corpora/stress/stress.tsv"))
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
            .map(|l| {
                let f: Vec<&str> = l.split('\t').collect();
                (f[0].to_string(), f[1].to_string())
            })
            .collect();
    let core =
        Pack::parse(&std::fs::read_to_string(root.join("packs/core.toml")).unwrap()).unwrap();
    let checker = Checker::new(vec![core]);
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let opts = Options {
        threads: 2,
        timeout: Duration::from_secs(60),
        ..Options::default()
    };
    let mut flagged = Vec::new();
    let mut full = 0;
    let mut ambiguous = Vec::new();
    for (_, sentence) in &rows {
        let a = analyze(&erg, sentence, Format::Plain, &opts);
        for d in checker.check(&erg, &a) {
            flagged.push(format!("{sentence}: {} {}", d.rule, d.message));
        }
        if a.sentences.iter().all(|s| s.strict()) {
            full += 1;
        }
        if a.sentences
            .iter()
            .any(|s| !report::ambiguity_lines(s).is_empty())
        {
            ambiguous.push(sentence.clone());
        }
    }
    assert!(flagged.is_empty(), "{}", flagged.join("\n"));
    // 54 of 58 as of 2026-10-05: "Can can can can can can.", "The prime
    // number few.", "The more you read, the more you know." and "The
    // sooner, the better." get fragments only.
    assert!(
        full >= 54,
        "only {full} of {} with a full analysis",
        rows.len()
    );
    for s in [
        "I saw the man with the telescope.",
        "The chicken is ready to eat.",
        "Time flies like an arrow.",
        "They are cooking apples.",
    ] {
        assert!(ambiguous.iter().any(|x| x == s), "not ambiguous: {s}");
    }
}
