//! Reads the whole vendored English Resource Grammar.

use emdysi_tdl::*;
use std::path::PathBuf;

fn erg() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../grammar/erg")
}

#[test]
fn loads_entire_erg() {
    let top = load(&erg().join("english.tdl"), Env::Type).unwrap();
    let count = |f: &dyn Fn(&Env) -> bool| top.entries.iter().filter(|e| f(&e.env)).count();
    let types = count(&|e| *e == Env::Type);
    let lex = count(&|e| *e == Env::Instance(Some("lex-entry".into())));
    let rules = count(&|e| *e == Env::Instance(Some("rule".into())));
    let lex_rules = count(&|e| *e == Env::Instance(Some("lex-rule".into())));
    let tmr = count(&|e| *e == Env::Instance(Some("token-mapping-rule".into())));
    eprintln!("types {types}, lex {lex}, rules {rules}, lex-rules {lex_rules}, tmr {tmr}");
    assert!(types > 5_000);
    assert!(lex > 30_000);
    assert!(rules > 200);
    assert!(lex_rules > 50);
    assert!(tmr > 100);
    assert!(top.letter_sets.len() >= 10);
    let inflected = top.entries.iter().filter(|e| e.def.affix.is_some()).count();
    assert!(inflected > 20);

    let mtr = load(&erg().join("mtr.tdl"), Env::Type).unwrap();
    assert!(!mtr.entries.is_empty());
}
