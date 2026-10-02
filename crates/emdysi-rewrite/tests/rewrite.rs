//! The guarded rewrite pipeline with a scripted stand-in for the model.

use emdysi_check::{Checker, Format, Options, Pack, analyze};
use emdysi_parse::{Erg, default_grammar_dir};
use emdysi_rewrite::{LanguageModel, RewriteOptions, Rewriter, Source, apply};

/// Returns its scripted rewrites in turn; prefers shorter text.
struct Scripted {
    replies: Vec<&'static str>,
    next: usize,
}

impl LanguageModel for Scripted {
    fn logprob(&mut self, _prefix: &str, text: &str) -> Option<(f64, usize)> {
        let n = text.split_whitespace().count();
        Some((-(text.len() as f64), n))
    }
    fn generate(&mut self, _: &str, _: &str, _: usize, _: f32, _: u32) -> Option<String> {
        let r = self
            .replies
            .get(self.next % self.replies.len())
            .map(|s| s.to_string());
        self.next += 1;
        r
    }
}

fn packs(names: &[&str]) -> Vec<Pack> {
    names
        .iter()
        .map(|p| {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../../packs/{p}.toml"));
            Pack::parse(&std::fs::read_to_string(path).unwrap()).unwrap()
        })
        .collect()
}

#[test]
fn guarded_rewrites() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = Checker::new(packs(&["core", "ai-tells"]));
    let options = Options {
        threads: 2,
        ..Options::default()
    };

    // Typo correction without a model: the spelling suggestion that the
    // grammar accepts.
    let src = "The resuls were clear.\n";
    let a = analyze(&erg, src, Format::Plain, &options);
    let d = checker.check(&erg, &a);
    let mut rw = Rewriter {
        erg: &erg,
        checker: &checker,
        options: options.clone(),
        rewrite: RewriteOptions::default(),
        model: None,
    };
    let props = rw.propose(&a, &d);
    let (out, n) = apply(&a, &props);
    assert_eq!((out.as_str(), n), ("The results were clear.\n", 1));

    // A grammatical error the rules cannot fix: the model's rewrites are
    // checked. One changes the meaning, one introduces an AI-tell word,
    // one is fine.
    let src = "He go to school every day.\n";
    let a = analyze(&erg, src, Format::Plain, &options);
    let d = checker.check(&erg, &a);
    let mut model = Scripted {
        replies: vec![
            "He went to the moon.",
            "He delves into school every day.",
            "He goes to school every day.",
        ],
        next: 0,
    };
    let mut rw = Rewriter {
        erg: &erg,
        checker: &checker,
        options: options.clone(),
        rewrite: RewriteOptions {
            samples: 2,
            ..RewriteOptions::default()
        },
        model: Some(&mut model),
    };
    let props = rw.propose(&a, &d);
    assert_eq!(props.len(), 1);
    let p = &props[0];
    let chosen = p.chosen.as_ref().expect("a rewrite");
    assert_eq!(chosen.text, "He goes to school every day.");
    assert_eq!(chosen.source, Source::Model);
    let why = |t: &str| {
        p.candidates
            .iter()
            .find(|c| c.text == t)
            .and_then(|c| c.rejected.clone())
            .unwrap_or_default()
    };
    assert!(why("He went to the moon.").contains("meaning"), "{p:#?}");
    assert!(
        why("He delves into school every day.").contains("introduces"),
        "{p:#?}"
    );
}
