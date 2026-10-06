//! The interpretations of each sentence given on the command line, with
//! their probabilities and dependencies.
//!
//! `cargo run --release -p emdysi-parse --example interpretations -- "I saw the man with the telescope."`

use emdysi_parse::{Erg, ambiguity, default_grammar_dir};

fn main() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    for s in std::env::args().skip(1) {
        let p = erg.parse(&s).unwrap();
        let t = p.temperature;
        let readings: Vec<_> = p
            .readings
            .into_iter()
            .filter(|r| emdysi_parse::is_strict(&r.root))
            .collect();
        println!("{s} ({} full readings)", readings.len());
        for i in ambiguity::interpretations(&readings, t, &s).iter().take(6) {
            println!(
                "  {:5.1}% ({} readings): {}",
                100.0 * i.probability,
                i.readings.len(),
                i.dependencies.iter().cloned().collect::<Vec<_>>().join(" ")
            );
        }
    }
}
