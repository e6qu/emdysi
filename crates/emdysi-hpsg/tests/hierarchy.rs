use emdysi_hpsg::*;
use std::time::Instant;

fn decl(name: &str, parents: &[&str]) -> (String, Vec<String>) {
    (name.into(), parents.iter().map(|p| p.to_string()).collect())
}

#[test]
fn glb_types_are_added() {
    // a and b share two maximal common subtypes c and d: needs a glb type.
    let h = Hierarchy::build(&[
        decl("a", &[]),
        decl("b", &[]),
        decl("c", &["a", "b"]),
        decl("d", &["a", "b"]),
        decl("e", &["a"]),
    ])
    .unwrap();
    let (a, b, c, d, e) = ["a", "b", "c", "d", "e"].map(|n| h.id(n).unwrap()).into();
    let g = h.glb(a, b).unwrap();
    assert!(h.is_glb_type(g));
    assert!(h.subsumed_by(c, g) && h.subsumed_by(d, g));
    assert_eq!(h.glb(c, d), None);
    assert_eq!(h.glb(e, b), None);
    assert_eq!(h.glb(e, a), Some(e));
    assert_eq!(h.glb(g, c), Some(c));
    assert_eq!(h.glb(TOP, d), Some(d));
}

#[test]
fn erg_hierarchy() {
    let erg =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../grammar/erg/english.tdl");
    let loaded = emdysi_tdl::load(&erg, emdysi_tdl::Env::Type).unwrap();
    let decls = type_declarations(&loaded.entries);
    let t = Instant::now();
    let h = Hierarchy::build(&decls).unwrap();
    eprintln!(
        "{} declared types, {} glb types, built in {:?}",
        h.n_declared(),
        h.len() - h.n_declared(),
        t.elapsed()
    );
    let list = h.id("*list*").unwrap();
    let cons = h.id("*cons*").unwrap();
    let null = h.id("*null*").unwrap();
    assert_eq!(h.glb(list, cons), Some(cons));
    assert_eq!(h.glb(cons, null), None);
    // Every pair of compatible types has a unique GLB: spot-check that the
    // GLB is subsumed by both and that no common subtype lies above it.
    let n = h.len() as TypeId;
    let mut checked = 0;
    for a in (0..n).step_by(37) {
        for b in (0..n).step_by(53) {
            if let Some(g) = h.glb(a, b) {
                assert!(h.subsumed_by(g, a) && h.subsumed_by(g, b));
                for &p in h.parents(g) {
                    assert!(
                        !(h.subsumed_by(p, a) && h.subsumed_by(p, b)),
                        "{} {} {}",
                        h.name(a),
                        h.name(b),
                        h.name(g)
                    );
                }
                checked += 1;
            }
        }
    }
    assert!(checked > 100);
}
