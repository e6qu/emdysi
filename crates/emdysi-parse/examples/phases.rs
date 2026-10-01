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
        if std::env::var("EDGES").is_ok() {
            use emdysi_hpsg::parser::{EdgeKind, EdgeState};
            let mut counts: std::collections::HashMap<(usize, usize, String), usize> =
                Default::default();
            for e in &r.chart {
                if e.state != EdgeState::Active || e.lexical {
                    continue;
                }
                if let EdgeKind::Rule(ri) = e.kind {
                    *counts
                        .entry((e.start, e.end, erg.rules[ri].name.clone()))
                        .or_default() += 1;
                }
            }
            let mut v: Vec<_> = counts.into_iter().collect();
            v.sort_by_key(|x| std::cmp::Reverse(x.1));
            let states = r.chart.iter().fold([0usize; 4], |mut acc, e| {
                acc[match e.state {
                    EdgeState::Active if e.lexical => 3,
                    EdgeState::Active => 0,
                    EdgeState::Packed(_) => 1,
                    EdgeState::Frozen => 2,
                }] += 1;
                acc
            });
            println!(
                "  active {} packed {} frozen {} lexical {}",
                states[0], states[1], states[2], states[3]
            );
            for ((a, b, n), c) in v.iter().take(15) {
                println!("    {a}-{b} {n}: {c}");
            }
            // Show where two same-span, same-rule active edges differ.
            if let Some(((a, b, n), _)) = v.first() {
                let same: Vec<&emdysi_hpsg::parser::Edge> = r
                    .chart
                    .iter()
                    .filter(|e| {
                        e.state == EdgeState::Active
                            && e.start == *a
                            && e.end == *b
                            && matches!(e.kind, EdgeKind::Rule(ri) if erg.rules[ri].name == *n)
                    })
                    .collect();
                let restr = erg.config.packing_restrictor.clone().unwrap();
                let g = &erg.grammar;
                for k in 1..4.min(same.len()) {
                    let x = same[0].dag.restrict(&restr);
                    let y = same[k].dag.restrict(&restr);
                    let mut out = Vec::new();
                    diff(
                        g,
                        &x,
                        &y,
                        0,
                        0,
                        &mut Vec::new(),
                        &mut out,
                        &mut std::collections::HashSet::new(),
                    );
                    println!(
                        "    diff 0 vs {k}: {}",
                        out.iter().take(8).cloned().collect::<Vec<_>>().join(" | ")
                    );
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn diff(
    g: &emdysi_hpsg::Grammar,
    a: &emdysi_hpsg::Dag,
    b: &emdysi_hpsg::Dag,
    x: u32,
    y: u32,
    path: &mut Vec<String>,
    out: &mut Vec<String>,
    seen: &mut std::collections::HashSet<(u32, u32)>,
) {
    if !seen.insert((x, y)) || out.len() > 20 {
        return;
    }
    if a.ty(x) != b.ty(y) {
        out.push(format!(
            "{}: {} vs {}",
            path.join("."),
            g.ts.name(a.ty(x)),
            g.ts.name(b.ty(y))
        ));
    }
    for &(f, v) in a.arcs(x) {
        if let Some(w) = b.arc(y, f) {
            path.push(g.feats.name(f).to_string());
            diff(g, a, b, v, w, path, out, seen);
            path.pop();
        }
    }
}
