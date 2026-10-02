//! The grammatical forms the `parallel` rules assign to each heading and
//! list item of Markdown files, with the root of the best analysis.
//!
//! `cargo run --release -p emdysi-check --example forms -- FILE.md...`
//!
//! Prints one TSV line per heading or list item: file, kind, best form,
//! possible forms, root of the best analysis, text; then counts of roots (strict, fragment, none) per kind.

use std::collections::BTreeMap;
use std::time::Duration;

use emdysi_check::structure::forms_of;
use emdysi_check::*;
use emdysi_parse::{Erg, default_grammar_dir};
use emdysi_text::blocks::BlockKind;

fn main() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let opts = Options {
        timeout: Duration::from_secs(10),
        ..Options::default()
    };
    let mut roots: BTreeMap<(String, String), usize> = BTreeMap::new();
    for path in std::env::args().skip(1) {
        let src = std::fs::read_to_string(&path).unwrap();
        let a = analyze(&erg, &src, Format::Markdown, &opts);
        let mut seen_items = std::collections::HashSet::new();
        for (si, s) in a.sentences.iter().enumerate() {
            let b = &a.blocks[s.block];
            let kind = match b.kind {
                BlockKind::Heading(_) => "heading",
                BlockKind::ListItem if seen_items.insert(b.item) => "item",
                _ => continue,
            };
            let root = match &s.parse {
                None => "skipped".to_string(),
                Some(p) => match p.readings.first() {
                    None => "none".to_string(),
                    Some(r) if emdysi_parse::is_strict(&r.root) => "strict".to_string(),
                    Some(r) => r.root.clone(),
                },
            };
            *roots.entry((kind.to_string(), root.clone())).or_default() += 1;
            let (best, possible) =
                forms_of(&erg, &a, si).map_or(("-".into(), String::new()), |f| {
                    let p: Vec<String> = f.possible.iter().map(|p| format!("{p:?}")).collect();
                    (format!("{:?}", f.best), p.join(","))
                });
            println!(
                "{path}\t{kind}\t{best}\t{possible}\t{root}\t{}",
                s.original.replace('\n', " ")
            );
        }
    }
    for ((k, r), n) in roots {
        eprintln!("{k}\t{r}\t{n}");
    }
}
