use std::time::Instant;
fn main() {
    let dir = emdysi_parse::default_grammar_dir();
    let t = Instant::now();
    let loaded = emdysi_tdl::load(&dir.join("english.tdl"), emdysi_tdl::Env::Type).unwrap();
    println!("tdl {:?}", t.elapsed());
    let t = Instant::now();
    let decls = emdysi_hpsg::type_declarations(&loaded.entries);
    let h = emdysi_hpsg::Hierarchy::build(&decls).unwrap();
    println!("hierarchy {:?} ({} types)", t.elapsed(), h.len());
    let t = Instant::now();
    let g = emdysi_hpsg::Grammar::compile(&loaded).unwrap();
    println!("grammar compile (incl. hierarchy again) {:?}", t.elapsed());
    drop(g);
    let t = Instant::now();
    let _erg = emdysi_parse::Erg::load(&dir).unwrap();
    println!("full Erg::load {:?}", t.elapsed());
}
