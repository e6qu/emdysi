use emdysi_hpsg::*;
use std::time::Instant;

#[test]
fn compile_erg() {
    let erg =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../grammar/erg/english.tdl");
    let loaded = emdysi_tdl::load(&erg, emdysi_tdl::Env::Type).unwrap();
    let t = Instant::now();
    let g = Grammar::compile(&loaded).unwrap();
    eprintln!("compiled in {:?}; {} errors", t.elapsed(), g.errors.len());
    for e in g.errors.iter().take(30) {
        eprintln!("  {e}");
    }
    let sizes: usize = (0..g.ts.hier.len() as TypeId)
        .map(|t| g.constraint(t).len())
        .sum();
    eprintln!("total constraint nodes {sizes}");
    let sign = g.ts.hier.id("sign").unwrap();
    eprintln!("sign: {} nodes", g.constraint(sign).len());
    let mut u = Unifier::new();
    let t = Instant::now();
    let mut ok = 0;
    let mut fail = 0;
    for inst in g
        .instances
        .iter()
        .filter(|i| i.status.as_deref() == Some("rule"))
    {
        match g.expand(&inst.body, &mut u) {
            Ok(_) => ok += 1,
            Err(e) => {
                fail += 1;
                eprintln!("rule {}: {e}", inst.name);
            }
        }
    }
    eprintln!("rules: {ok} ok, {fail} failed in {:?}", t.elapsed());
    let t = Instant::now();
    let (mut ok, mut fail) = (0, 0);
    for inst in g
        .instances
        .iter()
        .filter(|i| i.status.as_deref() == Some("lex-entry"))
    {
        match g.expand(&inst.body, &mut u) {
            Ok(_) => ok += 1,
            Err(e) => {
                fail += 1;
                if fail < 10 {
                    eprintln!("lex {}: {e}", inst.name);
                }
            }
        }
    }
    eprintln!("lex: {ok} ok, {fail} failed in {:?}", t.elapsed());
    assert_eq!(fail, 0);
    assert!(g.errors.is_empty());
}
