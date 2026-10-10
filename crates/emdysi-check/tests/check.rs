//! End-to-end checks with the bundled grammar and packs.

use emdysi_check::*;
use emdysi_parse::{Erg, default_grammar_dir};

fn packs() -> Vec<Pack> {
    [
        "core",
        "ai-tells",
        "plain-style",
        "substance",
        "structure",
        "terms",
    ]
    .iter()
    .map(|p| {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../packs/{p}.toml"));
        Pack::parse(&std::fs::read_to_string(path).unwrap()).unwrap()
    })
    .collect()
}

fn rules_at<'a>(src: &str, diags: &'a [Diagnostic], text: &str) -> Vec<&'a str> {
    let at = src.find(text).unwrap();
    diags
        .iter()
        .filter(|d| d.range.start <= at && at < d.range.end.max(d.range.start + 1))
        .map(|d| d.rule.as_str())
        .collect()
}

#[test]
fn checks_and_fixes() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = Checker::new(packs());
    let opts = Options {
        threads: 2,
        ..Options::default()
    };

    let src = "# Notes\n\nIt is important to note that we delve into the data. \
               The report was written by the the analysts.\n\n\
               We recieved the files. Run `cargo bild` now.\n";
    let a = analyze(&erg, src, Format::Markdown, &opts);
    assert_eq!(a.sentences.len(), 5);
    let d = checker.check(&erg, &a);

    assert!(rules_at(src, &d, "It is important").contains(&"ai-tells.signposting"));
    assert!(rules_at(src, &d, "delve").contains(&"ai-tells.vocabulary"));
    assert!(rules_at(src, &d, "written").contains(&"plain-style.passive"));
    assert!(rules_at(src, &d, "the the").contains(&"core.repeated-word"));
    assert!(rules_at(src, &d, "recieved").contains(&"core.spelling"));
    // Inline code is not spell-checked.
    assert!(!rules_at(src, &d, "bild").contains(&"core.spelling"));
    // Positions point into the source.
    let sp = d.iter().find(|x| x.rule == "core.spelling").unwrap();
    assert_eq!(&src[sp.range.clone()], "recieved");

    let (fixed, n) = apply_fixes(src, &d);
    assert!(n >= 2);
    assert!(fixed.contains("We received the files."));
    assert!(fixed.contains("by the analysts"));

    // "fast-paced" is not a passive clause.
    let src2 = "It was a fast-paced meeting.\n";
    let a2 = analyze(&erg, src2, Format::Plain, &opts);
    let d2 = checker.check(&erg, &a2);
    assert!(
        !d2.iter().any(|x| x.rule == "plain-style.passive"),
        "{d2:?}"
    );

    // Mixed American and British spelling: the majority wins.
    let src3 = "The colour of the theatre was grey. We analyzed the color.\n";
    let a3 = analyze(&erg, src3, Format::Plain, &opts);
    let d3 = checker.check(&erg, &a3);
    let c: Vec<&Diagnostic> = d3.iter().filter(|x| x.rule == "core.consistency").collect();
    assert_eq!(c.len(), 2, "{d3:?}");
    let (fixed3, _) = apply_fixes(src3, &c.into_iter().cloned().collect::<Vec<_>>());
    assert_eq!(
        fixed3,
        "The colour of the theatre was grey. We analysed the colour.\n"
    );

    // Named grammatical errors, from the grammar-error variant of the ERG.
    let src4 = "He go to school every day. We buyed a new car. The dog barked.\n";
    let a4 = analyze(&erg, src4, Format::Plain, &opts);
    let d4 = checker.check(&erg, &a4);
    assert!(
        rules_at(src4, &d4, "go to").contains(&"core.grammar-errors"),
        "{d4:?}"
    );
    assert!(
        rules_at(src4, &d4, "buyed").contains(&"core.grammar-errors"),
        "{d4:?}"
    );
    assert!(
        !rules_at(src4, &d4, "dog").contains(&"core.grammar-errors"),
        "{d4:?}"
    );

    // Checks on the semantics.
    let src5 = "This approach is better. The report was written. We did not see nothing. This shows that it works. The kitchen is larger than before.\n";
    let a5 = analyze(&erg, src5, Format::Plain, &opts);
    let d5 = checker.check(&erg, &a5);
    assert!(
        rules_at(src5, &d5, "better").contains(&"substance.missing-comparand"),
        "{d5:?}"
    );
    assert!(
        !rules_at(src5, &d5, "larger").contains(&"substance.missing-comparand"),
        "{d5:?}"
    );
    assert!(
        rules_at(src5, &d5, "written").contains(&"substance.agentless-passive"),
        "{d5:?}"
    );
    assert!(
        rules_at(src5, &d5, "not see").contains(&"plain-style.stacked-negation"),
        "{d5:?}"
    );
    assert!(
        rules_at(src5, &d5, "This shows").contains(&"substance.bare-demonstrative"),
        "{d5:?}"
    );
}

#[test]
fn glossary_terms() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let glossary = Pack::parse_glossary(
        r#"
[[concept]]
id = "sign-in"
definition = "Authenticating to an account."
term = [
  { text = "sign in", pos = "verb", status = "preferred" },
  { text = "log in", pos = "verb", status = "deprecated" },
]

[[concept]]
id = "javascript"
term = [{ text = "JavaScript" }]

[[concept]]
id = "cdp"
term = [{ text = "customer data platform", status = "preferred" }]

[[concept]]
id = "kubectl"
term = [{ text = "kubectl" }]
"#,
    )
    .unwrap();
    let mut packs = packs();
    packs.push(glossary);
    let checker = Checker::new(packs);
    let opts = Options {
        threads: 2,
        ..Options::default()
    };
    let src = "Log in to the console. We logged in twice. The Javascript client runs kubectl. \
               The customer data platform team approved it.\n";
    let a = analyze(&erg, src, Format::Plain, &opts);
    let d = checker.check(&erg, &a);
    let dep: Vec<&Diagnostic> = d.iter().filter(|x| x.rule == "terms.deprecated").collect();
    assert_eq!(dep.len(), 2, "{d:?}");
    assert_eq!(dep[0].replacement.as_deref(), Some("Sign in"));
    // Inflected: a suggestion, not a fix.
    assert_eq!(dep[1].replacement, None);
    assert_eq!(dep[1].suggestions, vec!["sign in".to_string()]);
    assert!(
        rules_at(src, &d, "Javascript").contains(&"terms.casing"),
        "{d:?}"
    );
    // Glossary words are known to spelling; a glossary term counts as one
    // noun in a stack.
    assert!(
        !rules_at(src, &d, "kubectl").contains(&"core.spelling"),
        "{d:?}"
    );
    assert!(
        !rules_at(src, &d, "customer data").contains(&"terms.noun-string"),
        "{d:?}"
    );
}

#[test]
fn existence_and_substitution() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let pack = Pack::parse(
        r#"
[pack]
name = "t"

[[rule]]
id = "t.avoid"
kind = "existence"
ignorecase = true
message = "Avoid '{match}'."
tokens = ["back ?end", "fubar"]
exceptions = ["Backend"]

[[rule]]
id = "t.wordy"
kind = "substitution"
ignorecase = true
fix = true
message = "Use '{replacement}' instead of '{match}'."
[rule.swap]
"in order to" = "to"
"utilize" = "use|employ"
"#,
    )
    .unwrap();
    let checker = Checker::new(vec![pack]);
    let opts = Options {
        threads: 2,
        ..Options::default()
    };
    let src = "In order to utilize the back end, read `fubar` and the Backend notes.\n";
    let a = analyze(&erg, src, Format::Markdown, &opts);
    let d = checker.check(&erg, &a);
    let avoid: Vec<&str> = d
        .iter()
        .filter(|x| x.rule == "t.avoid")
        .map(|x| &src[x.range.clone()])
        .collect();
    // Inline code and exceptions are skipped.
    assert_eq!(avoid, vec!["back end"], "{d:?}");
    let wordy: Vec<&Diagnostic> = d.iter().filter(|x| x.rule == "t.wordy").collect();
    assert_eq!(wordy.len(), 2, "{d:?}");
    assert_eq!(wordy[0].replacement.as_deref(), Some("To"));
    assert_eq!(wordy[0].message, "Use 'To' instead of 'In order to'.");
    // Several alternatives: suggestions only.
    assert_eq!(wordy[1].replacement, None);
    assert_eq!(wordy[1].suggestions, vec!["use", "employ"]);
}

#[test]
fn grammar_errors_precision() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = Checker::new(packs());
    let opts = Options {
        threads: 2,
        ..Options::default()
    };
    // Before: "Staying" and "Majesty" were reported as wrongly capitalized,
    // and the first item drew three made-up corrections.
    let src = "1. Keep going: Staying motivated and disciplined can be difficult, \
               especially if your dreams take a while.\n\n\
               Your Majesty, I write to you with a sense of urgency, as our \
               beloved kingdom is in danger.\n";
    let a = analyze(&erg, src, Format::Markdown, &opts);
    let d = checker.check(&erg, &a);
    assert!(!d.iter().any(|x| x.rule == "core.grammar-errors"), "{d:?}");
}

#[test]
fn modifier_kinds() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let pack = Pack::parse(
        r#"
[pack]
name = "t"

[[rule]]
id = "t.stack"
kind = "adjective-stack"
min = 3
message = "{count} adjectives"

[[rule]]
id = "t.density"
kind = "modifier-density"
min = 4
ratio = 0.25
"#,
    )
    .unwrap();
    let checker = Checker::new(vec![pack]);
    let opts = Options {
        threads: 2,
        ..Options::default()
    };
    let src = "The vibrant, confident, and independent women of today stand out. \
               The cat sat on the mat. \
               The gleaming silver moon cast an ethereal, shimmering glow over the tranquil, sleepy village.\n";
    let a = analyze(&erg, src, Format::Plain, &opts);
    let d = checker.check(&erg, &a);
    let stack: Vec<&str> = d
        .iter()
        .filter(|x| x.rule == "t.stack")
        .map(|x| &src[x.range.clone()])
        .collect();
    assert_eq!(
        stack,
        vec!["vibrant, confident, and independent women"],
        "{d:?}"
    );
    assert!(
        rules_at(src, &d, "gleaming").contains(&"t.density"),
        "{d:?}"
    );
    assert!(
        !rules_at(src, &d, "The cat").contains(&"t.density"),
        "{d:?}"
    );
}

#[test]
fn no_grammar_claims_on_labels_or_names() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = Checker::new(packs());
    let opts = Options {
        threads: 2,
        ..Options::default()
    };
    // A bold label that ends with a period is split off as a sentence of
    // its own; it is a label, not a clause missing its article.
    let src = "Testing comes in several forms.\n\n- **Remote moderated usability testing**. With web-based tools, you do not have to be in the same place as the participant.\n";
    let a = analyze(&erg, src, Format::Markdown, &opts);
    let d = checker.check(&erg, &a);
    assert!(!d.iter().any(|x| x.rule == "core.grammar-errors"), "{d:?}");
}

#[test]
fn grammar_corrections() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = Checker::new(packs());
    let opts = Options {
        threads: 2,
        ..Options::default()
    };
    // Each error with the correction the checker must suggest: the other
    // number of the same tense, the irregular past.
    for (src, word, fix) in [
        ("The results was clear.\n", "was", "were"),
        ("They doesn't know.\n", "doesn't", "don't"),
        ("He don't know.\n", "don't", "doesn't"),
        ("She are a writer.\n", "are", "is"),
        ("I buyed a car.\n", "buyed", "bought"),
        ("She runned home.\n", "runned", "ran"),
        ("He has went home.\n", "went", "gone"),
        ("She has wrote a letter.\n", "wrote", "written"),
        ("She can goes home.\n", "can goes", "can go"),
        ("He will makes it.\n", "will makes", "will make"),
        (
            "Policy objects describes the abstract logic of a policy.\n",
            "describes",
            "describe",
        ),
        (
            "Policy objects describes the abstract logic of a policy using Common Expression Language (CEL).\n",
            "describes",
            "describe",
        ),
        (
            "This tutorial show you how to install {{< glossary_tooltip term_id=\"dra\" text=\"DRA\" >}} drivers in your cluster.\n",
            "show",
            "shows",
        ),
        (
            "The function only needs to knows that there is some scope.\n",
            "to knows",
            "to know",
        ),
        (
            "We need more informations.\n",
            "informations",
            "information",
        ),
        (
            "The informations are useful.\n",
            "informations are",
            "information is",
        ),
        (
            "The informations help us.\n",
            "informations help",
            "information helps",
        ),
    ] {
        let a = analyze(&erg, src, Format::Plain, &opts);
        let d = checker.check(&erg, &a);
        let e = d
            .iter()
            .find(|x| x.rule == "core.grammar-errors" && &src[x.range.clone()] == word)
            .unwrap_or_else(|| panic!("{src}: {d:?}"));
        assert_eq!(e.suggestions, [fix], "{src}");
    }
}
