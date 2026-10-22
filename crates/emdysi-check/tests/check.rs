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
fn words_never_doubled_are_reported_whatever_the_parse() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = Checker::new(packs());
    let opts = Options {
        threads: 1,
        ..Options::default()
    };
    // The grammar has a full analysis for it (|helps you to| plus a
    // phrase |to stay secure|), but |to to| is a mistake.
    let src = "Following this recommendation helps you to to stay secure.\n";
    let a = analyze(&erg, src, Format::Plain, &opts);
    assert!(a.sentences[0].strict());
    let d = checker.check(&erg, &a);
    assert!(
        rules_at(src, &d, "to to").contains(&"core.repeated-word"),
        "{d:?}"
    );
    // Doublings English allows, and a particle before a preposition.
    for src in [
        "He said that that is true.\n",
        "Sign in in the top right corner.\n",
    ] {
        let a = analyze(&erg, src, Format::Plain, &opts);
        let d = checker.check(&erg, &a);
        assert!(
            !d.iter().any(|x| x.rule == "core.repeated-word"),
            "{src}: {d:?}"
        );
    }
}

#[test]
fn a_listed_misspelling_is_one_even_if_a_rare_list_has_it() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = Checker::new(packs());
    let opts = Options {
        threads: 1,
        ..Options::default()
    };
    // |homogenous| is in a rare word list and codespell's list of
    // misspellings; |quitted| is in codespell's list too, but the grammar
    // knows it (a period form), and |-ment| names a suffix.
    let src =
        "The nodes are homogenous. You quitted the house. Endings such as -ment are common.\n";
    let a = analyze(&erg, src, Format::Plain, &opts);
    let d = checker.check(&erg, &a);
    let at = rules_at(src, &d, "homogenous");
    assert!(at.contains(&"core.spelling"), "{d:?}");
    let fix = d.iter().find(|x| x.rule == "core.spelling").unwrap();
    assert!(fix.message.contains("homogeneous"), "{}", fix.message);
    assert!(
        !rules_at(src, &d, "quitted").contains(&"core.spelling"),
        "{d:?}"
    );
    assert!(
        !rules_at(src, &d, "ment").contains(&"core.spelling"),
        "{d:?}"
    );
}

#[test]
fn a_rare_word_codespell_lists_is_one_in_a_title_cased_name() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = Checker::new(packs());
    let opts = Options {
        threads: 1,
        ..Options::default()
    };
    // |manger| is a word, but in a name of common words it is a typo for
    // |manager|; on its own it stays a word.
    let src = "Next, the Memory Manger feature should be enabled. The ox ate from the manger.\n";
    let a = analyze(&erg, src, Format::Plain, &opts);
    let d = checker.check(&erg, &a);
    let flagged: Vec<_> = d.iter().filter(|x| x.rule == "core.spelling").collect();
    assert_eq!(flagged.len(), 1, "{d:?}");
    assert_eq!(&src[flagged[0].range.clone()], "Manger", "{d:?}");
    assert!(
        flagged[0].message.contains("manager"),
        "{}",
        flagged[0].message
    );
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
    // The plural of an acronym is a name, not a plural of a mass noun.
    let src4b = "PRs welcome!\n";
    let d4b = checker.check(&erg, &analyze(&erg, src4b, Format::Plain, &opts));
    assert!(
        !rules_at(src4b, &d4b, "PRs").contains(&"core.grammar-errors"),
        "{d4b:?}"
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
fn mentioned_people_options_and_plurals_of_ignored_words_are_not_typos() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = Checker::new(packs());
    let opts = Options {
        threads: 2,
        ..Options::default()
    };
    // Before: |epage| as a typo of |page|, |zulips| of |tulips| and
    // |funsigned| of |unsigned| (Rust blog).
    let src = "Comment by @epage:\n\n\
               Thanks to epage for the idea. We talked it over on zulips \
               durnig the week, and added an option like -funsigned-char.\n";
    let a = analyze(&erg, src, Format::Markdown, &opts);
    let d = checker.check(&erg, &a);
    let spelling: Vec<&str> = d
        .iter()
        .filter(|x| x.rule == "core.spelling")
        .map(|x| &src[x.range.clone()])
        .collect();
    assert_eq!(spelling, vec!["durnig"], "{d:?}");
}

#[test]
fn a_word_the_document_writes_as_code_is_not_a_typo() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = Checker::new(packs());
    let opts = Options {
        parse: false,
        ..Options::default()
    };
    // Before: |notin| as a typo of |noting| (Kubernetes docs), where the
    // document's examples write the operator as code.
    let src = "Restrict negative matching with the _notin_ operator, \
               as in `tier notin (frontend)`. Teh rest is simple.\n";
    let a = analyze(&erg, src, Format::Markdown, &opts);
    let d = checker.check(&erg, &a);
    let spelling: Vec<&str> = d
        .iter()
        .filter(|x| x.rule == "core.spelling")
        .map(|x| &src[x.range.clone()])
        .collect();
    assert_eq!(spelling, vec!["Teh"], "{d:?}");
}

#[test]
fn misspellings_the_morphology_or_the_model_used_to_hide() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = Checker::new(packs());
    let opts = Options {
        threads: 2,
        ..Options::default()
    };
    // Before: read as derived forms (|file| + -ed + -s, -ing on the noun
    // |specif|, |wit| + -in'), as |languages| + -s, or left to a model.
    // Real words stay unflagged, and so does a word glued to inline code.
    let src = "The fileds were witin reach, specifing the languagess. \
               The admistrators found it neccesary. \
               We built nightlies unfrequently, with two `let`s and grafana.\n";
    let a = analyze(&erg, src, Format::Markdown, &opts);
    let d = checker.check(&erg, &a);
    let spelling: Vec<&str> = d
        .iter()
        .filter(|x| x.rule == "core.spelling")
        .map(|x| &src[x.range.clone()])
        .collect();
    assert_eq!(
        spelling,
        vec![
            "fileds",
            "witin",
            "specifing",
            "languagess",
            "admistrators",
            "neccesary"
        ],
        "{d:?}"
    );
}

#[test]
fn a_lead_in_to_a_list_is_not_a_sentence() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = Checker::new(packs());
    let opts = Options {
        threads: 2,
        ..Options::default()
    };
    // Before: an agreement error on "are" (Kubernetes documentation).
    let src = "Typically, CRDs are a good fit if:\n\n\
               - you want to use the Kubernetes tools.\n\n\
               The dogs is happy.\n";
    let a = analyze(&erg, src, Format::Markdown, &opts);
    let d = checker.check(&erg, &a);
    let grammar: Vec<_> = d
        .iter()
        .filter(|x| x.rule == "core.grammar-errors")
        .collect();
    assert_eq!(grammar.len(), 1, "{d:?}");
    assert_eq!(&src[grammar[0].range.clone()], "is", "{d:?}");
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

#[test]
fn a_typo_of_a_word_the_document_uses() {
    // |cryptographic| is too rare for the common word lists, but the
    // document uses it, so |crytographic|, one edit away, is its typo.
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = Checker::new(packs());
    let opts = Options::default();
    let src = "We need crytographic machinery here. No other cryptographic tooling is required.\n";
    let a = analyze(&erg, src, Format::Plain, &opts);
    let d = checker.check(&erg, &a);
    let sp = d
        .iter()
        .find(|x| x.rule == "core.spelling")
        .unwrap_or_else(|| panic!("{d:?}"));
    assert_eq!(&src[sp.range.clone()], "crytographic");
    assert_eq!(sp.replacement.as_deref(), Some("cryptographic"));
}

#[test]
fn penn_tags_from_the_parse() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let opts = Options {
        threads: 1,
        ..Options::default()
    };
    let src = "The dogs quickly chased him to a bigger house, and Kim will stay.\n";
    let a = analyze(&erg, src, Format::Markdown, &opts);
    let r = a.sentences[0].best().expect("a reading");
    let tags: Vec<(String, &str)> = r
        .words
        .iter()
        .flat_map(|w| {
            emdysi_check::tags::penn_all(w)
                .into_iter()
                .map(|t| (w.surface.to_lowercase(), t))
        })
        .collect();
    let want = [
        ("the", "DT"),
        ("dogs", "NNS"),
        ("quickly", "RB"),
        ("chased", "VBD"),
        ("him", "PRP"),
        ("to", "TO"),
        ("a", "DT"),
        ("bigger", "JJR"),
        ("house", "NN"),
        ("and", "CC"),
        ("kim", "NNP"),
        ("will", "MD"),
        ("stay", "VB"),
    ];
    assert!(
        !tags.iter().any(|(s, t)| s == "," && *t == "CD"),
        "{tags:?}"
    );
    for (w, t) in want {
        assert!(
            tags.iter()
                .any(|(s, g)| s.trim_end_matches([',', '.']) == w && *g == t),
            "{w}/{t} not in {tags:?}"
        );
    }
}

#[test]
fn an_article_before_a_verb_form_is_not_fixed_automatically() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = Checker::new(packs());
    let opts = Options {
        threads: 1,
        ..Options::default()
    };
    // Rust book: "Instead of an creating a whole `extern` block", fixed by
    // dropping "an"; "a creating" would be wrong too.
    let src = "Instead of an creating a whole block, we add the keyword.\n";
    let a = analyze(&erg, src, Format::Markdown, &opts);
    let d: Vec<_> = checker
        .check(&erg, &a)
        .into_iter()
        .filter(|d| d.rule == "core.articles")
        .collect();
    assert_eq!(d.len(), 1, "{d:?}");
    assert_eq!(d[0].replacement, None);
    assert_eq!(d[0].suggestions, vec!["a".to_string()]);
    // Before a noun the fix stays automatic.
    let a = analyze(&erg, "It is an cluster.\n", Format::Markdown, &opts);
    let d: Vec<_> = checker
        .check(&erg, &a)
        .into_iter()
        .filter(|d| d.rule == "core.articles")
        .collect();
    assert_eq!(d[0].replacement.as_deref(), Some("a"), "{d:?}");
}

#[test]
fn two_capitals_take_the_article_of_their_first_letter() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = Checker::new(packs());
    let opts = Options {
        threads: 1,
        ..Options::default()
    };
    let flagged = |src: &str| {
        let a = analyze(&erg, src, Format::Markdown, &opts);
        checker
            .check(&erg, &a)
            .into_iter()
            .filter(|d| d.rule == "core.articles")
            .map(|d| d.replacement.unwrap_or_default())
            .collect::<Vec<_>>()
    };
    // Rust blog: "an MI 250X" (em-eye), not "mi" as the dictionary says.
    assert!(flagged("We covered an MI 250X and an RTX 4050.\n").is_empty());
    assert!(flagged("It is a US company.\n").is_empty());
    assert_eq!(flagged("It is an US company.\n"), vec!["a".to_string()]);
}

#[test]
fn run_on_clauses_at_a_pronoun() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = Checker::new(packs());
    let opts = Options {
        threads: 1,
        ..Options::default()
    };
    let found = |src: &str| {
        let a = analyze(&erg, src, Format::Markdown, &opts);
        checker
            .check(&erg, &a)
            .into_iter()
            .filter(|d| d.rule == "core.grammar-errors")
            .map(|d| (src[d.range.clone()].to_string(), d.suggestions.clone()))
            .collect::<Vec<_>>()
    };
    // The Rust book: a stray pronoun, deleted (not |let's; we dive|).
    assert_eq!(
        found(
            "To really understand it, let's we dive a little further into how the trait works.\n"
        ),
        vec![(" we".to_string(), vec![String::new()])]
    );
    // Two clauses: a semicolon, not |We fixed it shipped it|.
    assert_eq!(
        found("We fixed it we shipped it.\n"),
        vec![(" we".to_string(), vec!["; we".to_string()])]
    );
    assert!(found("We fixed it; we shipped it.\n").is_empty());
    // Not after a comma, nor after a conjunction (both from the held-out
    // set, where the semicolon was once suggested).
    assert!(found("Then, we can create a vector to hold that enum and so, ultimately, hold different types.\n").is_empty());
    assert!(found("Your lists will be easier to read if you:\n").is_empty());
    // Nor at the end (the novel; the Rust blog).
    assert!(found("But I dare not hope it.\n").is_empty());
    assert!(
        found("Tyler suggested adding this as a future possibility to RFC#3808 and FCPing it.\n")
            .is_empty()
    );
}

#[test]
fn grammar_error_corrections_are_words() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = Checker::new(packs());
    let opts = Options {
        threads: 1,
        ..Options::default()
    };
    // After the Rust compiler's development guide: |been| was corrected by
    // |beened|, an unknown word that parsed.
    let src = "Make sure that the places cannot been moved.\n";
    let a = analyze(&erg, src, Format::Markdown, &opts);
    for d in checker.check(&erg, &a) {
        assert!(
            !d.suggestions.iter().any(|s| s.contains("beened")),
            "{}: {:?}",
            d.rule,
            d.suggestions
        );
    }
}

#[test]
fn chunks_from_the_parse() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let opts = Options {
        threads: 1,
        ..Options::default()
    };
    for (text, want) in [
        (
            "The quick brown fox jumps over the lazy dogs.\n",
            vec![
                ("The", "B-NP-singular"),
                ("brown", "I-NP-singular"),
                ("fox", "E-NP-singular"),
                ("jumps", "B-VP"),
                ("over", "B-PP"),
                ("the", "B-NP-plural"),
                ("dogs", "E-NP-plural"),
            ],
        ),
        (
            "They have not been seen since.\n",
            vec![
                ("They", "B-NP-plural"),
                ("They", "E-NP-plural"),
                ("have", "B-VP"),
                ("not", "I-VP"),
                ("been", "I-VP"),
                ("seen", "I-VP"),
            ],
        ),
    ] {
        let a = analyze(&erg, text, Format::Plain, &opts);
        let best = a.sentences[0].best().unwrap();
        let chunks = emdysi_check::tags::chunks(&best.words);
        for (word, tag) in want {
            let i = best
                .words
                .iter()
                .position(|w| w.surface.trim_end_matches('.') == word)
                .unwrap_or_else(|| panic!("{word} in {text}"));
            assert!(
                chunks[i].iter().any(|c| c == tag),
                "{word}: {:?} in {text}",
                chunks[i]
            );
        }
    }
}

#[test]
fn an_antipattern_with_a_tag_condition() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let opts = Options {
        threads: 1,
        ..Options::default()
    };
    // Flags |run| except where the parse makes it a noun.
    let pack = Pack::parse(
        r#"
[pack]
name = "t"

[[rule]]
id = "t.run"
kind = "regex"
severity = "warning"
message = "run"
pattern = '(?i)\brun\b'
antipatterns = [{ pattern = '(?i)\b(?P<t1>run)\b', tags = ["t1=NN.*"] }]
"#,
    )
    .unwrap();
    let checker = Checker::new(vec![pack]);
    for (text, flagged) in [("I run every day.\n", true), ("The run was long.\n", false)] {
        let a = analyze(&erg, text, Format::Plain, &opts);
        let d = checker.check(&erg, &a);
        assert_eq!(
            d.iter().any(|x| x.rule == "t.run"),
            flagged,
            "{text}: {d:?}"
        );
    }
}

#[test]
fn a_mass_noun_has_languagetools_countability_tag() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let opts = Options {
        threads: 1,
        ..Options::default()
    };
    let a = analyze(&erg, "The information was useful.\n", Format::Plain, &opts);
    let best = a.sentences[0].best().unwrap();
    let w = best
        .words
        .iter()
        .find(|w| w.surface == "information")
        .unwrap();
    let tags = emdysi_check::tags::penn_all(w);
    assert!(
        tags.contains(&"NN") && tags.contains(&"NN:U"),
        "{tags:?} {}",
        w.le_type
    );
}
