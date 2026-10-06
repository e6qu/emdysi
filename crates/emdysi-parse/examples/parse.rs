use emdysi_parse::*;

fn main() {
    let t = std::time::Instant::now();
    let config = std::env::var("CONFIG").unwrap_or_else(|_| "ace/config.tdl".to_string());
    let mut erg = Erg::load_config(&default_grammar_dir(), &config).unwrap();
    for w in &erg.warnings {
        eprintln!("warning: {w}");
    }
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
    // NO_PACKING=1 turns ambiguity packing off, for comparison.
    if std::env::var_os("NO_PACKING").is_some() {
        erg.config.packing_restrictor = None;
    }
    // NO_BEAM=1 turns the per-cell beam off, for comparison.
    if std::env::var_os("NO_BEAM").is_some() {
        erg.config.cell_beam = None;
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
        // ROOT=<name> shows the readings under that root instead of the
        // best three.
        let root = std::env::var("ROOT").ok();
        let shown: Vec<_> = match &root {
            Some(n) => p.readings.iter().filter(|r| &r.root == n).take(3).collect(),
            None => p.readings.iter().take(3).collect(),
        };
        for r in shown {
            println!(
                "  [{}] {}\n      {}",
                r.root,
                r.tree.as_ref().map(|t| t.bracketed()).unwrap_or_default(),
                r.derivation
            );
        }
    }
}
