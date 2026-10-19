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
    // The versions of the sentence, the original first (after the question
    // that asks which word is wrong).
    let options = model.asked.iter().find(|o| o[0] == text.trim()).unwrap();
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

/// A model that scores text: surprised by `word` where it stands, and
/// prefers the version containing `want`; never asked to screen or locate.
struct Surprised {
    word: &'static str,
    surprise: f64,
    want: &'static str,
}

impl Ask for Surprised {
    fn ask(&mut self, context: &str, question: &str, options: &[&str]) -> Option<Vec<f64>> {
        assert!(
            !question.starts_with(emdysi_check::assist::SCREEN_QUESTION)
                && !question.starts_with(emdysi_check::assist::LOCATE_QUESTION),
            "{question}"
        );
        Prefers {
            want: self.want,
            asked: Vec::new(),
        }
        .ask(context, question, options)
    }

    fn surprise(
        &mut self,
        _context: &str,
        text: &str,
        spans: &[(usize, usize)],
    ) -> Option<Vec<f64>> {
        Some(
            spans
                .iter()
                .map(|&(f, t)| {
                    if &text[f..t] == self.word {
                        self.surprise
                    } else {
                        0.0
                    }
                })
                .collect(),
        )
    }
}

#[test]
fn a_surprising_word_screens_a_sentence_the_grammar_accepts() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let text = "He is interested on chess.\n";
    let a = analyze(&erg, text, Format::Markdown, &opts());
    let fixes = |model: &mut Surprised| -> Vec<Diagnostic> {
        core()
            .check_with(&erg, &a, Some(model))
            .into_iter()
            .filter(|d| d.rule == "core.grammar-fix")
            .collect()
    };
    let d = fixes(&mut Surprised {
        word: "on",
        surprise: 6.0,
        want: "interested in chess",
    });
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].replacement.as_deref(), Some("in"));
    // Below `screen_surprise`, the sentence is left alone.
    let d = fixes(&mut Surprised {
        word: "on",
        surprise: 1.0,
        want: "interested in chess",
    });
    assert!(d.is_empty(), "{d:?}");
}

#[test]
fn a_real_word_typo_is_corrected_at_the_word_named() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    // |Manger| is a word, so the spelling check passes it and the grammar
    // accepts the sentence; the model doubts it and names the word.
    let text = "The Controller Manger runs the controllers.\n";
    let a = analyze(&erg, text, Format::Markdown, &opts());
    struct Names;
    impl Ask for Names {
        fn ask(&mut self, _c: &str, question: &str, options: &[&str]) -> Option<Vec<f64>> {
            let want = if question.starts_with(emdysi_check::assist::SCREEN_QUESTION) {
                "no"
            } else if question.starts_with(emdysi_check::assist::LOCATE_QUESTION) {
                "Manger"
            } else {
                "Controller Manager runs"
            };
            Some(
                options
                    .iter()
                    .map(|o| if o.contains(want) { 0.95 } else { 0.05 })
                    .collect(),
            )
        }
    }
    let d: Vec<Diagnostic> = core()
        .check_with(&erg, &a, Some(&mut Names))
        .into_iter()
        .filter(|d| d.rule == "core.grammar-fix")
        .collect();
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(&text[d[0].range.clone()], "Manger");
    assert_eq!(d[0].replacement.as_deref(), Some("Manager"));
}

#[test]
fn a_sentence_without_a_strict_reading_gets_candidates() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    // Fragments only (its one full analysis reads |provides| as a verb of
    // saying with nothing quoted, which is ruled out): the candidates come
    // from its words and where its fragments meet.
    let text = "Binding objects link the policy to your cluster and provides scoping.\n";
    let a = analyze(&erg, text, Format::Markdown, &opts());
    assert!(!a.sentences[0].strict());
    assert!(
        a.sentences[0].best().is_some(),
        "no reading, not even fragments"
    );
    // The model names the word and prefers the correction.
    struct Names;
    impl Ask for Names {
        fn ask(&mut self, _c: &str, question: &str, options: &[&str]) -> Option<Vec<f64>> {
            let want = if question.starts_with(emdysi_check::assist::LOCATE_QUESTION) {
                "provides"
            } else {
                "and provide scoping"
            };
            Some(
                options
                    .iter()
                    .map(|o| if o.contains(want) { 0.95 } else { 0.05 })
                    .collect(),
            )
        }
    }
    let d: Vec<Diagnostic> = core()
        .check_with(&erg, &a, Some(&mut Names))
        .into_iter()
        .filter(|d| d.rule == "core.grammar-fix")
        .collect();
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(&text[d[0].range.clone()], "provides");
    assert_eq!(d[0].replacement.as_deref(), Some("provide"));
}

/// A stand-in model that names `word` as wrong and prefers the sentence
/// with `want`.
struct NamesAndPrefers {
    word: &'static str,
    want: &'static str,
}

impl Ask for NamesAndPrefers {
    fn ask(&mut self, _c: &str, question: &str, options: &[&str]) -> Option<Vec<f64>> {
        // It doubts the sentence when screened, too.
        let want = if question.starts_with(emdysi_check::assist::SCREEN_QUESTION) {
            "no"
        } else if question.starts_with(emdysi_check::assist::LOCATE_QUESTION) {
            self.word
        } else {
            self.want
        };
        Some(
            options
                .iter()
                .map(|o| if o.contains(want) { 0.95 } else { 0.05 })
                .collect(),
        )
    }
}

#[test]
fn a_missing_first_word_is_inserted_with_a_capital() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let text = "will generate tokens that look like `02b50b05`.\n";
    let a = analyze(&erg, text, Format::Markdown, &opts());
    let mut model = NamesAndPrefers {
        word: "will",
        want: "This will generate",
    };
    let d: Vec<Diagnostic> = core()
        .check_with(&erg, &a, Some(&mut model))
        .into_iter()
        .filter(|d| d.rule == "core.grammar-fix")
        .collect();
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].replacement.as_deref(), Some("This will"));
}

#[test]
fn a_sentence_adverb_written_as_its_adjective() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let text = "Additional, we are starting a new initiative.\n";
    let a = analyze(&erg, text, Format::Markdown, &opts());
    let mut model = NamesAndPrefers {
        word: "Additional",
        want: "Additionally, we",
    };
    let d: Vec<Diagnostic> = core()
        .check_with(&erg, &a, Some(&mut model))
        .into_iter()
        .filter(|d| d.rule == "core.grammar-fix")
        .collect();
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].replacement.as_deref(), Some("Additionally"));
}

#[test]
fn a_missing_object_at_the_end_is_inserted() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let text = "Remove the old CA from the files and the configs against.\n";
    let a = analyze(&erg, text, Format::Markdown, &opts());
    let mut model = NamesAndPrefers {
        word: "against",
        want: "configs against it.",
    };
    let d: Vec<Diagnostic> = core()
        .check_with(&erg, &a, Some(&mut model))
        .into_iter()
        .filter(|d| d.rule == "core.grammar-fix")
        .collect();
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].replacement.as_deref(), Some("against it"));
}

#[test]
fn corrections_of_two_words_and_related_words() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    // (sentence, the word the model names, the version it prefers, the
    // replacement)
    let cases = [
        (
            "The output is similar to this one.",
            "is",
            "output should be similar",
            "should be",
        ),
        (
            "The following is the example of the flow described in that document.",
            "of",
            "the example flow described",
            "",
        ),
        (
            "This flag is used with all the resources which has a status.",
            "which",
            "resources that have a status",
            "that have",
        ),
        // A negation (|doesn't|).
        (
            "If the spec of the Pod does already contain any secrets, the controller adds them.",
            "does",
            "Pod doesn't already contain",
            "doesn't",
        ),
        // Verified clause by clause: the whole corrected sentence has no
        // strict reading within its time, the clause with it does.
        (
            "Then we can create a vector to hold that enum and so, ultimately, holds different types.",
            "holds",
            "ultimately, hold different",
            "hold",
        ),
        (
            "The value will be included in the iteration produced by filter.",
            "iteration",
            "the iterator produced",
            "iterator",
        ),
    ];
    for (sentence, word, want, replacement) in cases {
        let text = format!("{sentence}\n");
        let a = analyze(&erg, &text, Format::Markdown, &opts());
        let mut model = NamesAndPrefers { word, want };
        let d: Vec<Diagnostic> = core()
            .check_with(&erg, &a, Some(&mut model))
            .into_iter()
            .filter(|d| d.rule == "core.grammar-fix")
            .collect();
        assert_eq!(d.len(), 1, "{sentence}: {d:?}");
        assert_eq!(d[0].replacement.as_deref(), Some(replacement), "{sentence}");
    }
}

#[test]
fn sentences_without_a_full_stop() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    // A list item the grammar accepts once it ends in a full stop is only
    // screened: the stand-in does not doubt it, so nothing else is asked.
    let a = analyze(&erg, "- Restart the kubelet\n", Format::Markdown, &opts());
    let mut model = Prefers {
        want: "Restarts",
        asked: Vec::new(),
    };
    let d = core().check_with(&erg, &a, Some(&mut model));
    assert!(!d.iter().any(|d| d.rule == "core.grammar-fix"), "{d:?}");
    assert_eq!(model.asked, vec![vec!["yes".to_string(), "no".to_string()]]);
    // An error in one is corrected as in a sentence with a full stop.
    let text = "- The quick brown fox jump over the crazy wall\n";
    let a = analyze(&erg, text, Format::Markdown, &opts());
    let mut model = Doubts { want: "fox jumps" };
    let d: Vec<Diagnostic> = core()
        .check_with(&erg, &a, Some(&mut model))
        .into_iter()
        .filter(|d| d.rule == "core.grammar-fix")
        .collect();
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].replacement.as_deref(), Some("jumps"));
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
    let text = "The scheduler wll start the pods.\n";
    let a = analyze(&erg, text, Format::Markdown, &opts());
    let spelling = |d: &[Diagnostic]| d.iter().find(|d| d.rule == "core.spelling").cloned();
    // Without a model: codespell's list knows the typo but offers two
    // words, |will| and |well|, so no fix.
    let plain = spelling(&checker.check(&erg, &a)).unwrap();
    assert_eq!(plain.replacement, None);
    assert!(plain.suggestions.len() >= 2, "{plain:?}");
    let mut model = Prefers {
        want: "scheduler will start",
        asked: Vec::new(),
    };
    let d = spelling(&checker.check_with(&erg, &a, Some(&mut model))).unwrap();
    assert_eq!(d.replacement.as_deref(), Some("will"));
    assert_eq!(d.suggestions[0], "will");
    assert!(d.message.contains("by the model"), "{}", d.message);
}

#[test]
fn a_word_two_edits_away_is_fixed_with_the_model() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = core();
    let text = "It is neccessry to read the docs.\n";
    let a = analyze(&erg, text, Format::Markdown, &opts());
    let spelling = |d: &[Diagnostic]| d.iter().find(|d| d.rule == "core.spelling").cloned();
    // |neccessry| is two edits from |necessary|, its only candidate, which
    // it only drops and adds letters to: a finding without a model, but no
    // fix, since a rare correct word can be as far from a common one.
    let plain = spelling(&checker.check(&erg, &a)).unwrap();
    assert_eq!(plain.replacement, None);
    assert_eq!(plain.suggestions, vec!["necessary".to_string()]);
    // A model that prefers the common word confirms it, with the fix.
    let mut model = Prefers {
        want: "is necessary to",
        asked: Vec::new(),
    };
    let d = spelling(&checker.check_with(&erg, &a, Some(&mut model))).unwrap();
    assert_eq!(d.replacement.as_deref(), Some("necessary"));
    // The word as written is one of the options; a model that prefers it
    // drops the finding.
    let mut model = Prefers {
        want: "is neccessry to",
        asked: Vec::new(),
    };
    assert_eq!(
        spelling(&checker.check_with(&erg, &a, Some(&mut model))),
        None
    );
}

#[test]
fn a_word_two_edits_from_several_words_needs_the_model() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = core();
    let text = "With the priaitized list feature, a second alternative can be specified.\n";
    let a = analyze(&erg, text, Format::Markdown, &opts());
    let spelling = |d: &[Diagnostic]| d.iter().find(|d| d.rule == "core.spelling").cloned();
    // |priaitized| is two edits from |prioritized| and from |privatized|:
    // no finding without a model ...
    assert_eq!(spelling(&checker.check(&erg, &a)), None);
    // ... and the model chooses among them.
    let mut model = Prefers {
        want: "the prioritized list",
        asked: Vec::new(),
    };
    let d = spelling(&checker.check_with(&erg, &a, Some(&mut model))).unwrap();
    assert_eq!(d.replacement.as_deref(), Some("prioritized"));
}

#[test]
fn a_typo_in_a_title_cased_name_needs_the_model() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = core();
    let text = "It may not be detected by the Node Shutdown Mannager in time.\n";
    let a = analyze(&erg, text, Format::Markdown, &opts());
    let spelling = |d: &[Diagnostic]| d.iter().find(|d| d.rule == "core.spelling").cloned();
    // A capitalized word is taken for a name, but among title-cased
    // common words (|Node Shutdown|) it may be a typo: only the model
    // can tell.
    assert_eq!(spelling(&checker.check(&erg, &a)), None);
    let mut model = Prefers {
        want: "Shutdown Manager in",
        asked: Vec::new(),
    };
    let d = spelling(&checker.check_with(&erg, &a, Some(&mut model))).unwrap();
    assert_eq!(d.replacement.as_deref(), Some("Manager"));
}

#[test]
fn a_three_letter_typo_needs_the_model() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = core();
    let text = "We wnt to read the logs.\n";
    let a = analyze(&erg, text, Format::Markdown, &opts());
    let spelling = |d: &[Diagnostic]| d.iter().find(|d| d.rule == "core.spelling").cloned();
    // Too short for a finding of its own, but one edit from |want|.
    assert_eq!(spelling(&checker.check(&erg, &a)), None);
    let mut model = Prefers {
        want: "We want to",
        asked: Vec::new(),
    };
    let d = spelling(&checker.check_with(&erg, &a, Some(&mut model))).unwrap();
    assert_eq!(d.replacement.as_deref(), Some("want"));
}

#[test]
fn a_listed_misspelling_is_fixed_without_the_model() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = core();
    let text = "It works like an assignement here.\n";
    let a = analyze(&erg, text, Format::Markdown, &opts());
    let spelling = |d: &[Diagnostic]| d.iter().find(|d| d.rule == "core.spelling").cloned();
    // |assignement| reads as |assignee| plus |-ment|, a coinage, and once
    // needed the model; codespell's list of known misspellings has it,
    // with a single correction: a fix without a model.
    let plain = spelling(&checker.check(&erg, &a)).unwrap();
    assert_eq!(plain.replacement.as_deref(), Some("assignment"));
    // A model has nothing to add.
    let mut model = Prefers {
        want: "an assignment here",
        asked: Vec::new(),
    };
    let d = spelling(&checker.check_with(&erg, &a, Some(&mut model))).unwrap();
    assert_eq!(d.replacement.as_deref(), Some("assignment"));
}
