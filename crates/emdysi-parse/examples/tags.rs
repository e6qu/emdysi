//! The part-of-speech tags the tagger gives each token of each line of
//! standard input.
//!
//! `echo "He go to school." | cargo run --release -p emdysi-parse --example tags`

use emdysi_parse::{Erg, default_grammar_dir};

fn main() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    for line in std::io::stdin().lines() {
        let line = line.unwrap();
        let toks = erg.tokens(&line);
        let shown: Vec<String> = toks
            .iter()
            .map(|t| {
                let tags: Vec<String> = t
                    .tags
                    .iter()
                    .map(|g| format!("{}:{:.2}", g.tag, g.prob))
                    .collect();
                format!("{}/{}", t.form, tags.join(","))
            })
            .collect();
        println!("{}", shown.join(" "));
    }
}
