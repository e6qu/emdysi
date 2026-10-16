//! How many candidate corrections the strict grammar accepts for each
//! sentence without a strict reading (one sentence per line on stdin):
//! the count, then the accepted corrections.
use emdysi_check::*;
use emdysi_parse::{Erg, default_grammar_dir};
use std::io::BufRead;
use std::time::{Duration, Instant};

fn main() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let opts = Options {
        threads: 1,
        ..Options::default()
    };
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let a = analyze(&erg, &line, Format::Plain, &opts);
        for s in &a.sentences {
            if s.strict() || s.parse.is_none() || !assist::WORDS.contains(&s.word_count()) {
                continue;
            }
            let start = Instant::now();
            let mut ok = Vec::new();
            for e in assist::edits(&erg, s).into_iter().take(80) {
                if start.elapsed() > Duration::from_secs(30) {
                    break;
                }
                if assist::verify(&erg, s, &e) {
                    ok.push(e.apply(&s.original));
                }
            }
            println!("{}\t{}\t{}", ok.len(), s.original.trim(), ok.join(" | "));
        }
    }
}
