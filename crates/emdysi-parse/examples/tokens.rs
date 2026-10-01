use emdysi_hpsg::chartmap::string_at;
use emdysi_parse::*;

fn main() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let g = &erg.grammar;
    for line in std::io::stdin().lines() {
        let line = line.unwrap();
        let toks: Vec<InputToken> = erg
            .repp
            .tokenize(&line)
            .into_iter()
            .map(|t| InputToken {
                form: t.form,
                from: t.from,
                to: t.to,
                tags: vec![Tag {
                    tag: "NN".into(),
                    prob: 1.0,
                }],
            })
            .collect();
        let t = std::time::Instant::now();
        let (lat, trace) = erg.map_tokens(&toks, true).unwrap();
        println!("{line}  ({:?})\n  fired: {}", t.elapsed(), trace.join(" "));
        let form = g.path("+FORM").unwrap();
        let class = g.path("+CLASS").unwrap();
        let uw = g.path("+TRAIT +UW").unwrap();
        let tag = g.path("+TNT +MAIN +TAG").unwrap();
        let mut alive: Vec<_> = lat.alive().collect();
        alive.sort_by(|a, b| lat.key(a.1.start).total_cmp(&lat.key(b.1.start)));
        for (_, e) in alive {
            let cls = e
                .dag
                .follow(0, &class)
                .map(|n| g.ts.name(e.dag.ty(n)))
                .unwrap_or_default();
            let uwv = e
                .dag
                .follow(0, &uw)
                .map(|n| g.ts.name(e.dag.ty(n)))
                .unwrap_or_default();
            println!(
                "  [{}-{}] {:?} class={} uw={} tag={:?}",
                lat.key(e.start),
                lat.key(e.end),
                string_at(&g.ts, &e.dag, &form).unwrap_or_default(),
                cls,
                uwv,
                string_at(&g.ts, &e.dag, &tag)
            );
        }
    }
}
