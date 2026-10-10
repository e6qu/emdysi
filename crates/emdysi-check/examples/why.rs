//! Why the grammar-error check does or does not claim an error, one
//! sentence per line of standard input: the parse, the readings of the
//! grammar-error variant and the errors they name, and the corrections
//! tried and verified.
//!
//! `echo "He will makes it." | cargo run --release -p emdysi-check --example why`

use emdysi_check::{Format, Options, analyze, rules::explain_grammar};
use emdysi_parse::{Erg, default_grammar_dir};

fn main() {
    let mut erg = Erg::load(&default_grammar_dir()).unwrap();
    if let Some(dir) = emdysi_parse::parse_cache_dir() {
        erg.cache_parses(&dir);
    }
    let opts = Options {
        threads: 1,
        ..Options::default()
    };
    for line in std::io::stdin().lines() {
        let line = line.unwrap();
        if line.trim().is_empty() {
            continue;
        }
        let a = analyze(&erg, &format!("{line}\n"), Format::Plain, &opts);
        for si in 0..a.sentences.len() {
            println!("== {}", a.sentences[si].original);
            println!("{}\n", explain_grammar(&erg, &a, si));
        }
    }
}
