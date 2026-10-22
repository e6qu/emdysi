//! A grammar read back from the cache is identical to a fresh compilation,
//! and is found without reading the sources while they are unchanged.

use emdysi_hpsg::*;

#[test]
fn cached_grammar_matches_fresh_compilation() {
    let erg =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../grammar/erg/english.tdl");
    let loaded = emdysi_tdl::load(&erg, emdysi_tdl::Env::Type).unwrap();
    let dir = std::env::temp_dir().join(format!("emdysi-cache-test-{}", std::process::id()));
    let fresh = Grammar::compile_cached(&loaded, Some(&dir)).unwrap();
    assert_eq!(
        std::fs::read_dir(&dir).unwrap().count(),
        1,
        "cache file written"
    );
    let cached = Grammar::compile_cached(&loaded, Some(&dir)).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();

    assert_eq!(fresh.ts.hier.len(), cached.ts.hier.len());
    assert_eq!(fresh.instances.len(), cached.instances.len());
    for t in 0..fresh.ts.hier.len() as TypeId {
        assert_eq!(fresh.ts.hier.name(t), cached.ts.hier.name(t));
        assert_eq!(
            fresh.display(fresh.constraint(t)),
            cached.display(cached.constraint(t))
        );
    }
    // Expanding an entry gives the same structure either way.
    let mut u = Unifier::new();
    let e = fresh.instance("dog_n1").unwrap();
    let a = fresh.expand(&e.body, &mut u).unwrap();
    let b = cached
        .expand(&cached.instance("dog_n1").unwrap().body, &mut u)
        .unwrap();
    assert_eq!(fresh.display(&a), cached.display(&b));
}

#[test]
fn a_grammar_whose_sources_are_unchanged_is_read_without_its_tdl() {
    // A small grammar of its own, so that a source can be edited.
    let src = std::env::temp_dir().join(format!("emdysi-from-cache-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&src);
    std::fs::create_dir_all(&src).unwrap();
    fn copy(from: &std::path::Path, to: &std::path::Path) {
        std::fs::create_dir_all(to).unwrap();
        for f in std::fs::read_dir(from).unwrap().flatten() {
            let p = f.path();
            if p.is_dir() {
                copy(&p, &to.join(f.file_name()));
            } else {
                std::fs::copy(&p, to.join(f.file_name())).unwrap();
            }
        }
    }
    copy(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../grammar/erg"),
        &src,
    );
    let dir = src.join("cache");
    let top = src.join("english.tdl");
    let loaded = emdysi_tdl::load(&top, emdysi_tdl::Env::Type).unwrap();
    // Nothing recorded yet: the sources must be read.
    assert!(Grammar::from_cache(&dir, "test").is_none());
    let fresh = Grammar::compile_cached(&loaded, Some(&dir)).unwrap();
    Grammar::record_sources(&dir, "test", &loaded.files);
    let cached = Grammar::from_cache(&dir, "test").expect("read from the cache");
    // Everything the TDL gave: the instances, as read, and the letter sets.
    assert_eq!(fresh.instances.len(), cached.instances.len());
    for (a, b) in fresh.instances.iter().zip(&cached.instances) {
        assert_eq!(a.name, b.name);
        assert_eq!(a.status, b.status);
        assert_eq!(a.body, b.body, "{}", a.name);
        assert_eq!(a.affix, b.affix, "{}", a.name);
        assert_eq!(a.docstrings, b.docstrings, "{}", a.name);
    }
    assert_eq!(fresh.letter_sets.len(), cached.letter_sets.len());
    for (a, b) in fresh.letter_sets.iter().zip(&cached.letter_sets) {
        assert_eq!((a.var, &a.chars, a.wild), (b.var, &b.chars, b.wild));
    }
    assert!(cached.instance("dog_n1").is_some());
    // An edited source makes the cache stale.
    let lexicon = loaded
        .files
        .iter()
        .find(|f| f.starts_with(&src) && f.file_name().is_some_and(|n| n == "lexicon.tdl"))
        .expect("lexicon.tdl read")
        .clone();
    let mut text = std::fs::read_to_string(&lexicon).unwrap();
    text.push_str("\n; edited\n");
    std::fs::write(&lexicon, text).unwrap();
    assert!(Grammar::from_cache(&dir, "test").is_none());
    std::fs::remove_dir_all(&src).unwrap();
}
