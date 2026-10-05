//! The grammar, with emdysi's extensions (`grammar/emdysi`), loads
//! cleanly: no inconsistent type, no rule that cannot be built.

use emdysi_parse::{Erg, MAL_CONFIG, default_grammar_dir};

#[test]
fn grammar_loads_without_warnings() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    assert!(erg.warnings.is_empty(), "{}", erg.warnings.join("\n"));
    let mal = Erg::load_config(&default_grammar_dir(), MAL_CONFIG).unwrap();
    assert!(mal.warnings.is_empty(), "{}", mal.warnings.join("\n"));
}
