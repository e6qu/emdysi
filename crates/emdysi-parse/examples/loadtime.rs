//! Time loading the grammar (run twice: the first run fills the cache).
use std::time::Instant;
fn main() {
    let t = Instant::now();
    let _erg = emdysi_parse::Erg::load(&emdysi_parse::default_grammar_dir()).unwrap();
    println!("Erg::load {:?}", t.elapsed());
}
