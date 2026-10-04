//! How often the grammar checks fire on the grammatical and ungrammatical
//! items of an [incr tsdb()] profile:
//! `cargo run --release -p emdysi-check --example errors_eval -- corpora/erg-gold/csli`.

use emdysi_check::*;
use emdysi_parse::{Erg, default_grammar_dir};
use std::process::Command;

fn main() {
    let dir = std::env::args().nth(1).expect("profile directory");
    let out = Command::new("zcat")
        .arg(format!("{dir}/item.gz"))
        .output()
        .unwrap();
    let items: Vec<(String, bool)> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.split('@').collect::<Vec<_>>())
        .filter(|f| f.len() > 10)
        .map(|f| (f[6].replace("\\s", "@"), f[10] == "1"))
        .collect();
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let pack = Pack::parse(include_str!("../../../packs/core.toml")).unwrap();
    let coverage = Pack::parse(include_str!("../../../packs/coverage.toml")).unwrap();
    let mut checker = Checker::new(vec![pack, coverage]);
    checker.disabled = vec!["core.spelling".into(), "core.consistency".into()];
    let opts = Options::default();
    let texts: Vec<&str> = items.iter().map(|(t, _)| t.as_str()).collect();
    // One document, one item per line.
    let src = texts.join("\n\n");
    let a = analyze(&erg, &src, Format::Markdown, &opts);
    let d = checker.check(&erg, &a);
    let mut counts = [[0usize; 3]; 2]; // [wf][named, generic, none]
    for (si, s) in a.sentences.iter().enumerate() {
        let wf = items
            .iter()
            .find(|(t, _)| t.trim() == s.original.trim())
            .map(|(_, w)| *w)
            .unwrap_or(true);
        let rules: Vec<&str> = d
            .iter()
            .filter(|x| x.sentence == Some(si))
            .map(|x| x.rule.as_str())
            .collect();
        let k = if rules.contains(&"core.grammar-errors") {
            0
        } else if rules.contains(&"coverage.grammar") {
            1
        } else {
            2
        };
        counts[wf as usize][k] += 1;
        if std::env::var("VERBOSE").is_ok() && (wf == (k == 0)) {
            let msgs: Vec<&str> = d
                .iter()
                .filter(|x| x.sentence == Some(si))
                .map(|x| x.message.as_str())
                .collect();
            println!("{} {} {:?}", if wf { "FP" } else { "--" }, s.original, msgs);
        }
    }
    for (wf, name) in [(0, "ungrammatical"), (1, "grammatical")] {
        let c = counts[wf];
        println!(
            "{name}: {} sentences; named error {}, generic 'possibly ungrammatical' {}, nothing {}",
            c.iter().sum::<usize>(),
            c[0],
            c[1],
            c[2]
        );
    }
}
