//! Print a path of the best reading's feature structure:
//! `echo "Abrams barked." | cargo run --release --example dagdump -- SYNSEM.LOCAL.CONT`.
use emdysi_parse::*;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_default();
    let mut erg = Erg::load(&default_grammar_dir()).unwrap();
    erg.keep_dags = true;
    for line in std::io::stdin().lines() {
        let p = erg.parse(&line.unwrap()).unwrap();
        let Some(r) = p.readings.first() else {
            continue;
        };
        let feats = erg
            .grammar
            .path(&path.replace('.', " "))
            .unwrap_or_default();
        let n = r.dag.as_ref().unwrap().follow(0, &feats).unwrap();
        let sub = sub_dag(r.dag.as_ref().unwrap(), n);
        println!("{}", erg.grammar.display(&sub));
    }
}

fn sub_dag(d: &emdysi_hpsg::Dag, n: u32) -> emdysi_hpsg::Dag {
    // Re-root by copying reachable nodes.
    let mut map = std::collections::HashMap::new();
    let mut order = vec![n];
    map.insert(n, 0u32);
    let mut i = 0;
    while i < order.len() {
        for &(_, v) in d.arcs(order[i]) {
            if let std::collections::hash_map::Entry::Vacant(e) = map.entry(v) {
                e.insert(order.len() as u32);
                order.push(v);
            }
        }
        i += 1;
    }
    let mut out = emdysi_hpsg::Dag {
        nodes: Vec::new(),
        arcs: Vec::new(),
    };
    for &o in &order {
        let start = out.arcs.len() as u32;
        for &(f, v) in d.arcs(o) {
            out.arcs.push((f, map[&v]));
        }
        out.nodes.push(emdysi_hpsg::dag::Node {
            ty: d.ty(o),
            arc_start: start,
            arc_len: d.arcs(o).len() as u32,
        });
    }
    out
}
