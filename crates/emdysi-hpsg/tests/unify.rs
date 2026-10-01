use emdysi_hpsg::*;
use emdysi_tdl::{Env, load_str};
use std::sync::Arc;

const TOY: &str = r#"
*avm* := *top*.
*list* := *avm*.
*cons* := *list* & [ FIRST *top*, REST *list* ].
*null* := *list*.
*diff-list* := *avm* & [ LIST *list*, LAST *list* ].
string := *top*.
bool := *avm*.
+ := bool.
- := bool.
agr := *avm* & [ NUM num, PER per ].
num := *avm*. sg := num. pl := num.
per := *avm*. 3rd := per. non3rd := per.
head := *avm* & [ AGR agr ].
noun := head & [ CASE bool ].
verb := head & [ AUX bool ].
sign := *avm* & [ HEAD head, ORTH *list* ].
a := *avm*. b := *avm*.
c := a & b & [ X bool ].
d := a & b & [ Y bool ].
"#;

fn grammar() -> Grammar {
    let loaded = load_str(TOY, "toy", Env::Type).unwrap();
    let g = Grammar::compile(&loaded).unwrap();
    assert!(g.errors.is_empty(), "{:?}", g.errors);
    g
}

fn desc(g: &Grammar, src: &str) -> Arc<Dag> {
    let loaded = load_str(&format!("x := {src}."), "t", Env::Instance(None)).unwrap();
    let mut u = Unifier::new();
    Arc::new(g.expand(&loaded.entries[0].def.body, &mut u).unwrap())
}

fn unify(g: &Grammar, a: &Arc<Dag>, b: &Arc<Dag>) -> Option<Dag> {
    let mut u = Unifier::new();
    u.begin();
    let ra = u.add(a.clone());
    let rb = u.add(b.clone());
    let cons = g.constraint_fn();
    if !u.unify(ra, rb, &g.ts, &cons) {
        return None;
    }
    u.copy(ra, &[])
}

#[test]
fn expansion_adds_type_constraints() {
    let g = grammar();
    let d = desc(&g, "sign & [ HEAD noun ]");
    let s = g.display(&d);
    assert!(s.contains("CASE bool"), "{s}");
    assert!(s.contains("AGR agr [ NUM num"), "{s}");
}

#[test]
fn feature_introduction_infers_types() {
    let g = grammar();
    // CASE is introduced by noun, so the HEAD value must be a noun.
    let d = desc(&g, "sign & [ HEAD.CASE + ]");
    assert!(g.display(&d).contains("HEAD noun"));
}

#[test]
fn reentrancy_propagates() {
    let g = grammar();
    let a = desc(&g, "sign & [ HEAD.AGR #x, ORTH < #x > ]");
    let b = desc(&g, "sign & [ HEAD.AGR.NUM sg ]");
    let r = unify(&g, &a, &b).unwrap();
    let s = g.display(&r);
    assert!(s.contains("FIRST #1"), "{s}");
    let head = r.follow(0, &g.path("HEAD AGR NUM").unwrap()).unwrap();
    let orth = r.follow(0, &g.path("ORTH FIRST NUM").unwrap()).unwrap();
    assert_eq!(head, orth);
    assert_eq!(g.ts.name(r.ty(orth)), "sg");
}

#[test]
fn clashes_fail() {
    let g = grammar();
    let a = desc(&g, "sign & [ HEAD noun ]");
    let b = desc(&g, "sign & [ HEAD verb ]");
    assert!(unify(&g, &a, &b).is_none());
    let a = desc(&g, "sign & [ HEAD.AGR #x, ORTH < #x > ]");
    let b = desc(&g, "sign & [ HEAD.AGR.NUM sg, ORTH < [ NUM pl ] > ]");
    assert!(unify(&g, &a, &b).is_none());
}

#[test]
fn glb_type_constraints_are_applied() {
    let g = grammar();
    // a & b has no declared GLB; the computed glbtype has subtypes c and d.
    // Unifying with c must add c's constraint (X bool).
    let x = Arc::new(Dag::atomic(g.ts.hier.id("a").unwrap()));
    let y = Arc::new(Dag::atomic(g.ts.hier.id("b").unwrap()));
    let r = unify(&g, &x, &y).unwrap();
    assert!(g.ts.hier.is_glb_type(r.root_type()));
    let c = desc(&g, "c");
    let r2 = unify(&g, &Arc::new(r), &c).unwrap();
    assert!(g.display(&r2).contains("X bool"));
}

#[test]
fn strings_unify_with_string_type() {
    let g = grammar();
    let a = desc(&g, r#"sign & [ ORTH < "dog" > ]"#);
    let b = desc(&g, "sign & [ ORTH < string > ]");
    let c = desc(&g, r#"sign & [ ORTH < "cat" > ]"#);
    assert!(unify(&g, &a, &b).is_some());
    assert!(unify(&g, &a, &c).is_none());
}

#[test]
fn cycles_are_rejected() {
    let g = grammar();
    let a = desc(&g, "*cons* & [ FIRST #1, REST #1 ]");
    let b = desc(&g, "*cons* & [ FIRST *cons* & [ REST #2 ], REST #2 ]");
    // FIRST = REST and FIRST.REST = REST, so REST = REST.REST.
    assert!(unify(&g, &a, &b).is_none());
    let c = desc(&g, "*cons* & [ FIRST *cons* ]");
    assert!(unify(&g, &a, &c).is_some());
}
