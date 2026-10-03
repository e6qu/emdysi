//! Rules and steps answered by a decision model, with a stand-in model.

use emdysi_check::decisions::{Ask, disambiguate};
use emdysi_check::*;
use emdysi_parse::{Erg, default_grammar_dir};

/// Answers by looking at the context and options.
struct Oracle<F: FnMut(&str, &str, &[&str]) -> Vec<f64>> {
    f: F,
    asked: usize,
}

impl<F: FnMut(&str, &str, &[&str]) -> Vec<f64>> Ask for Oracle<F> {
    fn ask(&mut self, context: &str, question: &str, options: &[&str]) -> Option<Vec<f64>> {
        self.asked += 1;
        Some((self.f)(context, question, options))
    }
}

fn pack() -> Pack {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/decisions.toml");
    Pack::parse(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn decide_rules() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = Checker::new(vec![pack()]);
    let src = "# Overview\n\nOur platform empowers teams to unlock their full potential in many ways.\n\nThe installer is 14 MB and needs Python 3.11 or later to run.\n";
    let opts = Options {
        threads: 2,
        ..Options::default()
    };
    let a = analyze(&erg, src, Format::Markdown, &opts);
    // Without a model, nothing.
    assert!(checker.check(&erg, &a).is_empty());
    // The stand-in finds sentences with a number checkable, and the
    // heading "Overview" uninformative.
    let mut oracle = Oracle {
        f: |context: &str, question: &str, options: &[&str]| {
            assert_eq!(options, ["yes", "no"]);
            let checkable = context.chars().any(|c| c.is_ascii_digit());
            let yes = if question.contains("could check") {
                if checkable { 0.9 } else { 0.05 }
            } else if question.contains("heading") {
                if context == "Overview" { 0.1 } else { 0.9 }
            } else {
                0.5
            };
            vec![yes, 1.0 - yes]
        },
        asked: 0,
    };
    let d = checker.check_with(&erg, &a, Some(&mut oracle));
    let rules: Vec<(&str, &str)> = d
        .iter()
        .map(|d| (d.rule.as_str(), &src[d.range.clone()]))
        .collect();
    assert!(
        rules.contains(&("decisions.vague-heading", "Overview")),
        "{rules:?}"
    );
    assert!(
        rules
            .iter()
            .any(|(r, t)| *r == "decisions.unverifiable" && t.starts_with("Our platform")),
        "{rules:?}"
    );
    assert!(
        !rules.iter().any(|(_, t)| t.starts_with("The installer")),
        "{rules:?}"
    );
    assert!(oracle.asked > 0);
}

#[test]
fn readings_settled_by_the_model() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let src = "The relationship between sleep and memory is intricate.\n";
    let opts = Options {
        threads: 1,
        ..Options::default()
    };
    let mut a = analyze(&erg, src, Format::Plain, &opts);
    let readings = &a.sentences[0].parse.as_ref().unwrap().readings;
    assert!(readings.len() >= 2);
    let before = readings[0].derivation.clone();
    // Prefers the grouping "[sleep and memory]" wherever it is offered.
    let mut oracle = Oracle {
        f: |_: &str, _: &str, options: &[&str]| {
            let good = |o: &str| o.contains("[sleep and memory]");
            match (good(options[0]), good(options[1])) {
                (true, false) => vec![0.9, 0.1],
                (false, true) => vec![0.1, 0.9],
                _ => vec![0.5, 0.5],
            }
        },
        asked: 0,
    };
    let changed = disambiguate(&mut a, &mut oracle, f64::INFINITY, 0.6);
    let readings = &a.sentences[0].parse.as_ref().unwrap().readings;
    if changed == 1 {
        assert_ne!(readings[0].derivation, before);
    } else {
        // The best reading already groups "sleep and memory".
        assert_eq!(changed, 0);
        assert_eq!(readings[0].derivation, before);
    }
    assert_eq!(oracle.asked, 1);
}
