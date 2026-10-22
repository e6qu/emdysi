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
    // One directory per grammar, with a parse of each sentence in it.
    fn bins(d: &std::path::Path) -> usize {
        std::fs::read_dir(d)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .map(|p| {
                if p.is_dir() {
                    bins(&p)
                } else {
                    usize::from(p.extension().is_some_and(|x| x == "bin"))
                }
            })
            .sum()
    }
    let versions = std::fs::read_dir(&dir).unwrap().count();
    assert_eq!(versions, 2);
    assert_eq!(bins(&dir), 4);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn pruning_keeps_the_most_recently_used() {
    use std::time::{Duration, SystemTime};
    let dir = std::env::temp_dir().join(format!("emdysi-parse-prune-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    // Ten parses of 1,000 bytes, each used a minute after the one before,
    // in two versions; and a file that is not a parse.
    let now = SystemTime::now();
    let mut files = Vec::new();
    for i in 0..10u64 {
        let sub = dir.join(if i % 2 == 0 { "v1/ab" } else { "v2/cd" });
        std::fs::create_dir_all(&sub).unwrap();
        let f = sub.join(format!("{i}.bin"));
        std::fs::write(&f, [0u8; 1000]).unwrap();
        let t = now - Duration::from_secs(60 * (10 - i));
        std::fs::File::options()
            .append(true)
            .open(&f)
            .unwrap()
            .set_modified(t)
            .unwrap();
        files.push(f);
    }
    std::fs::write(dir.join("v1/used"), b"").unwrap();
    // Under the limit: nothing goes.
    emdysi_parse::prune_parse_cache(&dir, 10_000);
    assert!(files.iter().all(|f| f.exists()));
    // Over it: the least recently used go, down to four fifths of it.
    emdysi_parse::prune_parse_cache(&dir, 5_000);
    let kept: Vec<bool> = files.iter().map(|f| f.exists()).collect();
    assert_eq!(kept, [vec![false; 6], vec![true; 4]].concat());
    assert!(dir.join("v1/used").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_embedded_grammar_is_the_repositorys() {
    let cache = std::env::temp_dir().join(format!("emdysi-embedded-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&cache);
    let dir = emdysi_parse::embedded_grammar_in(&cache).unwrap();
    assert!(dir.starts_with(&cache));
    let repo = default_grammar_dir();
    for rel in ["english.tdl", "ace/config.tdl", "../emdysi/settings.cfg"] {
        assert_eq!(
            std::fs::read(dir.join(rel)).unwrap(),
            std::fs::read(repo.join(rel)).unwrap(),
            "{rel}"
        );
    }
    // Written once: a second call finds it.
    assert_eq!(emdysi_parse::embedded_grammar_in(&cache).unwrap(), dir);
    let erg = Erg::load(&dir).unwrap();
    assert!(!erg.parse("The cat sleeps.").unwrap().readings.is_empty());
    let _ = std::fs::remove_dir_all(&cache);
}
