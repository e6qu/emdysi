//! How often the best reading is the hand-judged one
//! (`corpora/ai-treebank/treebank.tsv`), with each sentence analysed in its
//! document, before and after settling close calls from the rest of the
//! document ([`emdysi_check::decisions::prefer_document_phrases`]):
//! `cargo run --release -p emdysi-check --example treebank_eval -- [MARGIN...]`.
//! With `VERBOSE=1`, also each sentence, right or wrong, and each change.

use std::collections::HashMap;
use std::path::Path;

use emdysi_check::decisions::prefer_document_phrases;
use emdysi_check::*;
use emdysi_parse::{Erg, default_grammar_dir};

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(o) => out.push(o),
            None => out.push('\\'),
        }
    }
    out
}

/// The skeleton of a derivation: `(name start end children...)` without
/// the token strings, as the treebank stores it.
fn skeleton(derivation: &str) -> String {
    let mut out = String::new();
    let mut in_string = false;
    let mut depth_of_string = Vec::new();
    let chars: Vec<char> = derivation.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_string {
            if c == '\\' {
                i += 1;
            } else if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match c {
            '"' => in_string = true,
            '(' => {
                // A list holding only a string is a token: drop it.
                let rest: String = chars[i + 1..].iter().collect();
                if rest.trim_start().starts_with('"') {
                    let mut depth = 0;
                    let mut j = i;
                    let mut quoted = false;
                    while j < chars.len() {
                        match chars[j] {
                            '\\' if quoted => j += 1,
                            '"' => quoted = !quoted,
                            '(' if !quoted => depth += 1,
                            ')' if !quoted => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                        j += 1;
                    }
                    i = j + 1;
                    depth_of_string.push(());
                    continue;
                }
                out.push('(');
            }
            _ => out.push(c),
        }
        i += 1;
    }
    // Normalise spaces.
    let s = out.split_whitespace().collect::<Vec<_>>().join(" ");
    s.replace(" )", ")")
}

fn main() {
    let margins: Vec<f64> = std::env::args()
        .skip(1)
        .filter_map(|m| m.parse().ok())
        .collect();
    let margins = if margins.is_empty() {
        vec![1.0, 2.0, 4.0]
    } else {
        margins
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tb = std::fs::read_to_string(root.join("corpora/ai-treebank/treebank.tsv")).unwrap();
    // Gold by source and sentence.
    let mut by_source: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for l in tb.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = l.split('\t').collect();
        if f.len() >= 4 && f[3] != "-" {
            by_source
                .entry(f[1].to_string())
                .or_default()
                .push((f[2].to_string(), f[3].to_string()));
        }
    }
    let beemo: HashMap<String, String> =
        std::fs::read_to_string(root.join("corpora/beemo/sample.tsv"))
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with('#'))
            .filter_map(|l| {
                let f: Vec<&str> = l.split('\t').collect();
                Some((f.first()?.to_string(), unescape(f.get(3)?)))
            })
            .collect();
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let opts = Options {
        threads: 4,
        ..Options::default()
    };
    let mut total = 0;
    let mut base = 0;
    let mut after = vec![0; margins.len()];
    let mut changed = vec![0; margins.len()];
    let mut sources: Vec<&String> = by_source.keys().collect();
    sources.sort();
    for src in sources {
        let doc = match src.split_once('/') {
            Some(("ai-prose", f)) => {
                std::fs::read_to_string(root.join("corpora/ai-prose").join(f)).unwrap()
            }
            Some(("beemo", id)) => beemo[id].clone(),
            _ => continue,
        };
        let a = analyze(&erg, &doc, Format::Markdown, &opts);
        let right = |a: &Analysis| {
            by_source[src]
                .iter()
                .filter(|(sent, gold)| {
                    a.sentences.iter().any(|s| {
                        s.original.trim() == sent
                            && s.best().is_some_and(|r| skeleton(&r.derivation) == *gold)
                    })
                })
                .count()
        };
        if std::env::var("VERBOSE").is_ok() {
            for (sent, gold) in &by_source[src] {
                let best = a
                    .sentences
                    .iter()
                    .find(|s| s.original.trim() == sent)
                    .and_then(|s| s.best())
                    .map(|r| skeleton(&r.derivation));
                let ok = best.as_deref() == Some(gold.as_str());
                println!("{} {sent}", if ok { "RIGHT" } else { "WRONG" });
                if !ok {
                    println!("  gold {gold}\n  best {}", best.unwrap_or_default());
                }
            }
        }
        total += by_source[src].len();
        base += right(&a);
        for (k, &m) in margins.iter().enumerate() {
            let mut b = a.clone();
            changed[k] += prefer_document_phrases(&mut b, m);
            if std::env::var("VERBOSE").is_ok() {
                for (sent, gold) in &by_source[src] {
                    let pick = |a: &Analysis| {
                        a.sentences
                            .iter()
                            .find(|s| s.original.trim() == sent)
                            .and_then(|s| s.best())
                            .map(|r| skeleton(&r.derivation))
                    };
                    let (x, y) = (pick(&a), pick(&b));
                    if x != y {
                        let ok = |v: &Option<String>| {
                            if v.as_deref() == Some(gold.as_str()) {
                                "right"
                            } else {
                                "wrong"
                            }
                        };
                        println!("CHANGED margin {m}: {} -> {}: {sent}", ok(&x), ok(&y));
                    }
                }
            }
            after[k] += right(&b);
        }
    }
    println!("{total} judged sentences in their documents; best reading right: {base}");
    for (k, m) in margins.iter().enumerate() {
        println!(
            "  document phrases, margin {m}: {} right ({} sentences changed)",
            after[k], changed[k]
        );
    }
}
