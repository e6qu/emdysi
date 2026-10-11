//! Every reading of each sentence on standard input (root, score,
//! derivation), one sentence per block, with the time taken: for checking
//! that a change to the parser leaves its results exactly as they were.
//!
//! `cargo run --release -p emdysi-parse --example dump < sentences.txt`

use emdysi_parse::*;
use std::time::Instant;

fn main() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let mut total = 0.0;
    for line in std::io::stdin().lines() {
        let line = line.unwrap();
        let t = Instant::now();
        let p = erg.parse(&line).unwrap();
        let secs = t.elapsed().as_secs_f64();
        total += secs;
        println!("# {line}");
        println!("readings {} exhausted {}", p.readings.len(), p.exhausted);
        for r in &p.readings {
            println!("{} {:.6} {}", r.root, r.score, r.derivation);
        }
        eprintln!("{secs:8.3}s {line}");
    }
    eprintln!("total {total:.1}s");
}
