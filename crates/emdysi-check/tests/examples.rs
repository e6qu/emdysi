//! Every rule's `examples` must draw a diagnostic from that rule, and its
//! `acceptable` documents must not.

use std::path::Path;
use std::time::Duration;

use emdysi_check::*;
use emdysi_parse::{Erg, default_grammar_dir};

#[test]
fn rule_examples() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "toml"))
        .collect();
    files.sort();
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let opts = Options {
        threads: 2,
        timeout: Duration::from_secs(60),
        ..Options::default()
    };
    let mut failures = Vec::new();
    let mut checked = 0;
    let packs: Vec<Pack> = files
        .iter()
        .map(|f| Pack::parse(&std::fs::read_to_string(f).unwrap()).unwrap())
        .collect();
    for pack in &packs {
        for rule in &pack.rules {
            // A rule over other rules' findings runs with all of them.
            let checker = if matches!(rule.kind, emdysi_check::rules::Kind::TellDensity { .. }) {
                Checker::new(packs.clone())
            } else {
                Checker::new(vec![Pack {
                    name: pack.name.clone(),
                    description: String::new(),
                    rules: vec![rule.clone()],
                    concepts: pack.concepts.clone(),
                }])
            };
            // A pattern over the text needs no parse.
            let opts = Options {
                parse: !matches!(rule.kind, emdysi_check::rules::Kind::Regex { .. }),
                ..opts.clone()
            };
            for (doc, want) in rule
                .examples
                .iter()
                .map(|d| (d, true))
                .chain(rule.acceptable.iter().map(|d| (d, false)))
            {
                checked += 1;
                let a = analyze(&erg, doc, Format::Markdown, &opts);
                let got: Vec<Diagnostic> = checker
                    .check(&erg, &a)
                    .into_iter()
                    .filter(|d| d.rule == rule.id)
                    .collect();
                if got.is_empty() == want {
                    failures.push(format!(
                        "{}: {} {doc:?}{}",
                        rule.id,
                        if want { "missed" } else { "flagged" },
                        got.iter()
                            .map(|d| format!("\n    {:?}: {}", &doc[d.range.clone()], d.message))
                            .collect::<String>()
                    ));
                }
            }
        }
    }
    assert!(checked > 40, "only {checked} examples");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
