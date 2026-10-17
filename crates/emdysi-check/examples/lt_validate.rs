//! The rules of a pack whose examples hold with the parse: each
//! `examples` document draws a finding from the rule and no `acceptable`
//! one does. Prints the ids of the rules that pass, one a line.
//!
//! For LanguageTool's rules with part-of-speech conditions, which need the
//! parse and are too many to check in the test suite:
//!
//! ```sh
//! python3 scripts/import-languagetool.py --candidates /tmp/cand.toml
//! cargo run --release -p emdysi-check --example lt_validate /tmp/cand.toml \
//!     > scripts/languagetool-tagged.txt
//! python3 scripts/import-languagetool.py
//! ```
//!
//! `THREADS` (default 3) sets the number of rules checked at once; parses
//! are cached between runs (see `Erg::cache_parses`).

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use emdysi_check::*;
use emdysi_parse::{Erg, default_grammar_dir};

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: lt_validate PACK.toml");
    let pack = Pack::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let mut erg = Erg::load(&default_grammar_dir()).unwrap();
    if let Some(dir) = emdysi_parse::parse_cache_dir() {
        erg.cache_parses(&dir);
    }
    let opts = Options {
        threads: 1,
        timeout: Duration::from_secs(20),
        ..Options::default()
    };
    let threads: usize = std::env::var("THREADS")
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or(3);
    let next = AtomicUsize::new(0);
    let passed: Mutex<Vec<(usize, String)>> = Mutex::default();
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(rule) = pack.rules.get(i) else { break };
                    let checker = Checker::new(vec![Pack {
                        name: pack.name.clone(),
                        description: String::new(),
                        rules: vec![rule.clone()],
                        concepts: Vec::new(),
                    }]);
                    let holds = rule
                        .examples
                        .iter()
                        .map(|d| (d, true))
                        .chain(rule.acceptable.iter().map(|d| (d, false)))
                        .all(|(doc, want)| {
                            let a = analyze(&erg, doc, Format::Markdown, &opts);
                            let found = checker.check(&erg, &a).iter().any(|d| d.rule == rule.id);
                            found == want
                        });
                    if holds {
                        passed.lock().unwrap().push((i, rule.id.clone()));
                    }
                    if i % 50 == 0 {
                        eprintln!("{i} of {}", pack.rules.len());
                    }
                }
            });
        }
    });
    let mut passed = passed.into_inner().unwrap();
    passed.sort();
    for (_, id) in &passed {
        println!("{id}");
    }
    eprintln!("{} of {} rules pass", passed.len(), pack.rules.len());
}
