//! Grammaticality judgments on minimal pairs: for each pair, whether the
//! grammar gives the acceptable sentence a strict analysis and the
//! unacceptable one none.
//!
//! `cargo run --release -p emdysi-check --example minimal_pairs -- corpora/blimp/sample.tsv`
//!
//! The input is TSV: paradigm, field, phenomenon, good sentence, bad
//! sentence (as in `corpora/blimp/sample.tsv`); lines starting with `#`
//! are skipped. Reports, per paradigm and overall, how often the good
//! sentence is accepted, the bad one rejected, and both (pair accuracy).

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Duration;

use emdysi_parse::{Erg, default_grammar_dir, is_strict};

fn main() {
    let path = std::env::args().nth(1).expect("TSV file");
    let src = std::fs::read_to_string(&path).unwrap();
    let pairs: Vec<(String, String, String)> = src
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f.len() >= 5).then(|| (f[0].to_string(), f[3].to_string(), f[4].to_string()))
        })
        .collect();
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let accepts = |s: &str| {
        erg.parse_limited(s, Duration::from_secs(10), 20)
            .is_ok_and(|p| p.readings.iter().any(|r| is_strict(&r.root)))
    };
    // paradigm -> [pairs, good accepted, bad rejected, both]
    let stats: Mutex<BTreeMap<String, [usize; 4]>> = Mutex::new(BTreeMap::new());
    let next = Mutex::new(0usize);
    std::thread::scope(|s| {
        for _ in 0..std::thread::available_parallelism().map_or(2, |n| n.get()) {
            s.spawn(|| {
                loop {
                    let i = {
                        let mut n = next.lock().unwrap();
                        *n += 1;
                        *n - 1
                    };
                    let Some((uid, good, bad)) = pairs.get(i) else {
                        break;
                    };
                    let g = accepts(good);
                    let b = !accepts(bad);
                    let mut st = stats.lock().unwrap();
                    let e = st.entry(uid.clone()).or_default();
                    e[0] += 1;
                    e[1] += g as usize;
                    e[2] += b as usize;
                    e[3] += (g && b) as usize;
                    if std::env::var("VERBOSE").is_ok() && !(g && b) {
                        println!(
                            "{uid}\t{}{}\t{good}\t{bad}",
                            if g { "" } else { "good-rejected " },
                            if b { "" } else { "bad-accepted" }
                        );
                    }
                }
            });
        }
    });
    let st = stats.into_inner().unwrap();
    let pct = |a: usize, n: usize| 100.0 * a as f64 / n.max(1) as f64;
    let mut total = [0usize; 4];
    for (uid, s) in &st {
        println!(
            "{uid:45} {:4} pairs  good accepted {:5.1}%  bad rejected {:5.1}%  pair {:5.1}%",
            s[0],
            pct(s[1], s[0]),
            pct(s[2], s[0]),
            pct(s[3], s[0])
        );
        for k in 0..4 {
            total[k] += s[k];
        }
    }
    println!(
        "TOTAL {} pairs: good accepted {:.1}%, bad rejected {:.1}%, pair accuracy {:.1}%",
        total[0],
        pct(total[1], total[0]),
        pct(total[2], total[0]),
        pct(total[3], total[0])
    );
}
