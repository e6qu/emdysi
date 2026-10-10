//! Cached parses read back exactly as they were parsed, for the grammar
//! and its grammar-error variant.

use emdysi_parse::{Erg, Parse, default_grammar_dir};

/// The two parses are the same. Semantics are compared as values: their
/// variable properties are a hash map, printed in no fixed order.
fn same(a: &Parse, b: &Parse, text: &str) {
    let strip = |p: &Parse| {
        let mut p = p.clone();
        for r in &mut p.readings {
            r.mrs = None;
        }
        format!("{p:?}")
    };
    assert_eq!(strip(a), strip(b), "{text}");
    for (x, y) in a.readings.iter().zip(&b.readings) {
        assert_eq!(x.mrs, y.mrs, "{text}");
    }
}

#[test]
fn cached_parses_round_trip() {
    let dir = std::env::temp_dir().join(format!("emdysi-parse-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut erg = Erg::load(&default_grammar_dir()).unwrap();
    erg.cache_parses(&dir);
    for text in [
        "I saw the man with the telescope.",
        "The results was clear.",
    ] {
        let first = erg.parse(text).unwrap();
        let again = erg.parse(text).unwrap();
        same(&first, &again, text);
        assert!(again.readings.iter().any(|r| r.mrs.is_some()), "{text}");
        let mal = erg.mal().unwrap();
        let first = mal.parse(text).unwrap();
        let again = mal.parse(text).unwrap();
        same(&first, &again, text);
    }
    // Two parses of each sentence per grammar: four entries.
    let entries = std::fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .flat_map(|d| std::fs::read_dir(d.path()).unwrap().flatten())
        .count();
    assert_eq!(entries, 4);
    let _ = std::fs::remove_dir_all(&dir);
}
