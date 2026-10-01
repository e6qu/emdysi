use emdysi_hpsg::*;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let erg =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../grammar/erg/english.tdl");
    let loaded = emdysi_tdl::load(&erg, emdysi_tdl::Env::Type).unwrap();
    let g = Grammar::compile(&loaded).unwrap();
    for name in &args[1..] {
        if let Some(t) = g.ts.hier.id(name) {
            let d = g.constraint(t);
            println!("type {name} ({} nodes):\n{}\n", d.len(), g.display(d));
        } else if let Some(i) = g.instance(name) {
            let mut u = Unifier::new();
            let d = g.expand(&i.body.clone(), &mut u).unwrap();
            println!("instance {name} ({} nodes):\n{}\n", d.len(), g.display(&d));
        }
    }
}
