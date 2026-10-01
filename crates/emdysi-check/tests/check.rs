//! End-to-end checks with the bundled grammar and packs.

use emdysi_check::*;
use emdysi_parse::{Erg, default_grammar_dir};

fn packs() -> Vec<Pack> {
    ["core", "ai-tells", "plain-style"]
        .iter()
        .map(|p| {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../../packs/{p}.toml"));
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
    assert!(rules_at(src, &d, "the the").contains(&"plain-style.repeated-word"));
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
}
