use emdysi_hpsg::Unifier;
use emdysi_hpsg::parser::Parser;
use emdysi_parse::*;
use std::time::Instant;

fn main() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    for line in std::io::stdin().lines() {
        let line = line.unwrap();
        let t = Instant::now();
        let tokens = erg.tokens(&line);
        let t1 = t.elapsed();
        let (lat, _) = erg.map_tokens(&tokens, false).unwrap();
        let t2 = t.elapsed();
        let mut u = Unifier::new();
        let items = erg.lexicon.instantiate(&erg.grammar, &lat, &mut u);
        let t3 = t.elapsed();
        let n = items.len();
        let r = Parser::new(
            &erg.grammar,
            &erg.rules,
            &erg.qc,
            &erg.config,
            &erg.lexical_filtering,
        )
        .parse(&lat, items);
        let t4 = t.elapsed();
        println!(
            "{line}\n  repp {t1:?} tmr {:?} lex {:?} ({n} items, {} alive tokens) parse {:?} ({} edges, {} readings, {} lex filtered)",
            t2 - t1,
            t3 - t2,
            lat.alive().count(),
            t4 - t3,
            r.chart.len(),
            r.readings.len(),
            r.filtered_lexical
        );
        println!("  {:?}", r.stats);
    }
}
