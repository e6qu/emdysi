//! Spelling and grammar fixes chosen by a decision model among corrections
//! the grammar verified (`assist`), with a stand-in model that answers as
//! a good one would.

use std::time::Duration;

use emdysi_check::decisions::Ask;
use emdysi_check::*;
use emdysi_parse::{Erg, default_grammar_dir};

/// Prefers the option containing `want`; asked questions are recorded.
struct Prefers {
    want: &'static str,
    asked: Vec<Vec<String>>,
}

impl Ask for Prefers {
    fn ask(&mut self, _context: &str, _question: &str, options: &[&str]) -> Option<Vec<f64>> {
        self.asked
            .push(options.iter().map(|o| o.to_string()).collect());
        let hits = options.iter().filter(|o| o.contains(self.want)).count();
        Some(
            options
                .iter()
                .map(|o| {
                    if hits > 0 && o.contains(self.want) {
                        0.95 / hits as f64
                    } else {
                        0.05 / (options.len() - hits).max(1) as f64
                    }
                })
                .collect(),
        )
    }
}

fn core() -> Checker {
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/core.toml"),
    )
    .unwrap();
    Checker::new(vec![Pack::parse(&src).unwrap()])
}

fn opts() -> Options {
    Options {
        threads: 1,
        timeout: Duration::from_secs(30),
        ..Options::default()
    }
}

#[test]
fn grammar_fix_needs_the_grammar_and_the_model() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = core();
    let text = "The quick brown fox jump over the crazy wall.\n";
    let a = analyze(&erg, text, Format::Markdown, &opts());
    // Without a model: the grammar reads it as a noun phrase (a jump), so
    // it claims nothing.
    assert!(
        !checker
            .check(&erg, &a)
            .iter()
            .any(|d| d.rule.starts_with("core.grammar")),
    );
    // With one: the corrections offered are those the grammar accepts
    // (no |foxs|), and the model's choice is the fix.
    let mut model = Prefers {
        want: "fox jumps",
        asked: Vec::new(),
    };
    let d: Vec<Diagnostic> = checker
        .check_with(&erg, &a, Some(&mut model))
        .into_iter()
        .filter(|d| d.rule == "core.grammar-fix")
        .collect();
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(&text[d[0].range.clone()], "jump");
    assert_eq!(d[0].replacement.as_deref(), Some("jumps"));
    let options = &model.asked[0];
    assert_eq!(options[0], text.trim());
    assert!(options.iter().all(|o| !o.contains("foxs")), "{options:?}");
    // A model that prefers the original: no claim.
    let mut model = Prefers {
        want: "fox jump over",
        asked: Vec::new(),
    };
    assert!(
        !checker
            .check_with(&erg, &a, Some(&mut model))
            .iter()
            .any(|d| d.rule == "core.grammar-fix")
    );
}

/// Doubts every sentence it is asked to screen, and prefers the option
/// containing `want`.
struct Doubts {
    want: &'static str,
}

impl Ask for Doubts {
    fn ask(&mut self, context: &str, question: &str, options: &[&str]) -> Option<Vec<f64>> {
        if question.starts_with(emdysi_check::assist::SCREEN_QUESTION) {
            return Some(vec![0.1, 0.9]);
        }
        Prefers {
            want: self.want,
            asked: Vec::new(),
        }
        .ask(context, question, options)
    }
}

#[test]
fn an_error_the_grammar_accepts_needs_the_model_to_doubt_it() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let text = "He is interested on chess.\n";
    let a = analyze(&erg, text, Format::Markdown, &opts());
    assert!(a.sentences[0].strict());
    let mut model = Doubts {
        want: "interested in chess",
    };
    let d: Vec<Diagnostic> = core()
        .check_with(&erg, &a, Some(&mut model))
        .into_iter()
        .filter(|d| d.rule == "core.grammar-fix")
        .collect();
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(&text[d[0].range.clone()], "on");
    assert_eq!(d[0].replacement.as_deref(), Some("in"));
}

#[test]
fn grammatical_sentences_are_only_screened() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let a = analyze(
        &erg,
        "The quick brown fox jumps over the lazy dog.\n",
        Format::Markdown,
        &opts(),
    );
    let mut model = Prefers {
        want: "jumped",
        asked: Vec::new(),
    };
    let d = core().check_with(&erg, &a, Some(&mut model));
    assert!(d.is_empty(), "{d:?}");
    // One question: is it correct? The stand-in does not doubt it, so no
    // corrections are tried.
    assert_eq!(model.asked, vec![vec!["yes".to_string(), "no".to_string()]]);
}

#[test]
fn spelling_fix_chosen_in_context() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = core();
    let text = "We need to chck the logs.\n";
    let a = analyze(&erg, text, Format::Markdown, &opts());
    let spelling = |d: &[Diagnostic]| d.iter().find(|d| d.rule == "core.spelling").cloned();
    // Without a model: several words are one edit away, so no fix.
    let plain = spelling(&checker.check(&erg, &a)).unwrap();
    assert_eq!(plain.replacement, None);
    assert!(plain.suggestions.len() >= 2);
    let mut model = Prefers {
        want: "to check the",
        asked: Vec::new(),
    };
    let d = spelling(&checker.check_with(&erg, &a, Some(&mut model))).unwrap();
    assert_eq!(d.replacement.as_deref(), Some("check"));
    assert_eq!(d.suggestions[0], "check");
    assert!(d.message.contains("by the model"), "{}", d.message);
}
