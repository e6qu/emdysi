//! A grammar read back from the cache is identical to a fresh compilation.

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
