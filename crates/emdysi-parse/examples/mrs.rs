//! Print the MRS of the best reading of each input line:
//! `echo "Abrams barked." | cargo run --release --example mrs`.
use emdysi_parse::*;

fn main() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    for line in std::io::stdin().lines() {
        let p = erg.parse(&line.unwrap()).unwrap();
        if let Some(m) = p.readings.first().and_then(|r| r.mrs.as_ref()) {
            println!("{}", m.to_simple());
            for e in m.problems() {
                println!("  problem: {e}");
            }
        }
    }
}
