//! The MRS read out of the gold derivation equals the gold MRS recorded in
//! the ERG's `mrs` test suite (up to variable names).

use emdysi_hpsg::mrs::{Mrs, isomorphic};
use emdysi_parse::*;

#[path = "../examples/common/tsdb.rs"]
mod tsdb;
use tsdb::*;

#[test]
fn mrs_matches_gold() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let dir = default_grammar_dir().join("../../corpora/erg-gold/mrs");
    let (mut checked, mut failures) = (0, Vec::new());
    for it in items(dir.to_str().unwrap()).into_iter().take(60) {
        let (Some(gold), Some(gold_mrs)) = (&it.gold, &it.mrs) else {
            continue;
        };
        let p = erg.parse(&it.text).unwrap();
        let Some(r) = p
            .readings
            .iter()
            .find(|r| parse_sexp(&r.derivation).is_some_and(|t| &skeleton(&t, false) == gold))
        else {
            continue;
        };
        checked += 1;
        let ours = r.mrs.as_ref().expect("an MRS");
        let theirs = Mrs::parse_simple(gold_mrs).expect("gold MRS parses");
        if !isomorphic(ours, &theirs) {
            failures.push(format!(
                "{}\n  gold: {gold_mrs}\n  ours: {}",
                it.text,
                ours.to_simple()
            ));
        }
        assert!(
            ours.problems().is_empty(),
            "{}: {:?}",
            it.text,
            ours.problems()
        );
    }
    assert!(checked >= 50, "only {checked} items checked");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
