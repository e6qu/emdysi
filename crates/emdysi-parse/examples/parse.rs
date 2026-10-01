use emdysi_parse::*;

fn main() {
    let t = std::time::Instant::now();
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    eprintln!("loaded in {:?}", t.elapsed());
    for line in std::io::stdin().lines() {
        let line = line.unwrap();
        let p = erg.parse(&line).unwrap();
        println!(
            "{line}\n  {} readings, {} lexical items, {} edges{}, {:?}",
            p.readings.len(),
            p.lexical_items,
            p.edges,
            if p.exhausted { " (exhausted)" } else { "" },
            p.elapsed
        );
        for r in p.readings.iter().take(3) {
            println!(
                "  [{}] {}\n      {}",
                r.root,
                r.tree.bracketed(),
                r.derivation
            );
        }
    }
}
