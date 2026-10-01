use emdysi_parse::*;

fn main() {
    let t = std::time::Instant::now();
    let mut erg = Erg::load(&default_grammar_dir()).unwrap();
    if let Some(b) = std::env::var("CELL_BEAM")
        .ok()
        .and_then(|b| b.parse::<usize>().ok())
    {
        erg.config.cell_beam = (b > 0).then_some(b);
    }
    if let Some(n) = std::env::var("BEAM_FROM")
        .ok()
        .and_then(|b| b.parse::<usize>().ok())
    {
        erg.config.cell_beam_from = n;
    }
    if let Some(n) = std::env::var("MAX_READINGS")
        .ok()
        .and_then(|b| b.parse::<usize>().ok())
    {
        erg.config.max_readings = n;
    }
    if let Some(t) = std::env::var("TIMEOUT")
        .ok()
        .and_then(|b| b.parse::<u64>().ok())
    {
        erg.config.timeout = std::time::Duration::from_secs(t);
    }
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
                r.tree.as_ref().map(|t| t.bracketed()).unwrap_or_default(),
                r.derivation
            );
        }
    }
}
