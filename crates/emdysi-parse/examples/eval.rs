//! Evaluate against [incr tsdb()] gold profiles:
//! `cargo run --release --example eval -- corpora/erg-gold/mrs [limit]`.
//!
//! Reports coverage on grammatical items, rejection of ungrammatical ones,
//! and how often the gold derivation is among the readings.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use emdysi_parse::*;

#[path = "common/tsdb.rs"]
mod tsdb;
use tsdb::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = &args[1];
    let limit: usize = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(usize::MAX);
    let mut erg = Erg::load(&default_grammar_dir()).unwrap();
    // Parses are cached between runs (see `Erg::cache_parses`).
    if let Some(dir) = emdysi_parse::parse_cache_dir() {
        erg.cache_parses(&dir);
    }
    erg.config.timeout = Duration::from_secs(20);
    erg.config.max_edges = 60_000;
    // CELL_BEAM=0 disables chart pruning, for comparison.
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

    let items: Vec<(String, String, bool)> = read_relation(dir, "item")
        .into_iter()
        .filter(|f| f.len() > 10)
        .map(|f| (f[0].clone(), f[6].clone(), f[10] == "1"))
        .take(limit)
        .collect();
    let parse_to_item: HashMap<String, String> = read_relation(dir, "parse")
        .into_iter()
        .filter(|f| f.len() > 2)
        .map(|f| (f[0].clone(), f[2].clone()))
        .collect();
    let mut gold: HashMap<String, String> = HashMap::new();
    let mut gold_mrs: HashMap<String, String> = HashMap::new();
    for f in read_relation(dir, "result") {
        if f.len() > 10 {
            if let (Some(item), Some(tree)) = (parse_to_item.get(&f[0]), parse_sexp(&f[10])) {
                gold.insert(item.clone(), skeleton(&tree, true));
                if let Some(m) = f.get(13).filter(|m| !m.is_empty()) {
                    gold_mrs.insert(item.clone(), m.clone());
                }
            }
        }
    }
    let mrs_checked = std::sync::atomic::AtomicUsize::new(0);
    let mrs_ok = std::sync::atomic::AtomicUsize::new(0);

    let results = Mutex::new(Vec::new());
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
                    let Some((id, text, wf)) = items.get(i) else {
                        break;
                    };
                    let mut p = erg.parse(text).unwrap();
                    // A fragment cover is not an analysis.
                    p.readings.retain(|r| r.root != "fragment");
                    let ours: Vec<String> = p
                        .readings
                        .iter()
                        .filter_map(|r| {
                            parse_sexp(&r.derivation).map(|t| skeleton(&erg_names(t), false))
                        })
                        .collect();
                    let g = gold.get(id).cloned();
                    let hit = g.as_ref().is_some_and(|g| ours.iter().any(|o| o == g));
                    // Semantics of the reading with the gold derivation.
                    if let (Some(g), Some(gm)) = (&g, gold_mrs.get(id)) {
                        if let Some(k) = ours.iter().position(|o| o == g) {
                            let ours_mrs = p.readings[k].mrs.as_ref();
                            let gold_m = emdysi_hpsg::mrs::Mrs::parse_simple(gm);
                            mrs_checked.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            let same = match (ours_mrs, &gold_m) {
                                (Some(a), Some(b)) => emdysi_hpsg::mrs::isomorphic(a, b),
                                _ => false,
                            };
                            if same {
                                mrs_ok.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            } else if std::env::var("MRS_VERBOSE").is_ok() {
                                println!(
                                    "MRS {id} {text}\n  gold: {gm}\n  ours: {}",
                                    ours_mrs.map(|m| m.to_simple()).unwrap_or_default()
                                );
                            }
                        }
                    }
                    results.lock().unwrap().push((
                        id.clone(),
                        text.clone(),
                        *wf,
                        p.readings.len(),
                        hit,
                        g.is_some(),
                        p.exhausted,
                        p.elapsed,
                        ours.first().cloned(),
                        g,
                    ));
                }
            });
        }
    });
    let mut results = results.into_inner().unwrap();
    results.sort_by_key(|r| r.0.parse::<u64>().unwrap_or(0));
    // ITEMS=<file> appends one line per item: id, grammatical, parsed,
    // gold found, sentence, gold ranked first.
    if let Ok(path) = std::env::var("ITEMS") {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        for r in &results {
            let first = r.9.is_some() && r.8 == r.9;
            writeln!(
                f,
                "{}\t{}\t{}\t{}\t{}\t{first}",
                r.0,
                r.2,
                r.3 > 0,
                r.4,
                r.1
            )
            .unwrap();
        }
    }
    let (
        mut wf,
        mut wf_parsed,
        mut nwf,
        mut nwf_parsed,
        mut with_gold,
        mut gold_hit,
        mut exhausted,
        mut top1,
    ) = (0, 0, 0, 0, 0, 0, 0, 0);
    let mut total = Duration::ZERO;
    for (id, text, w, n, hit, has_gold, exh, el, first, g) in &results {
        total += *el;
        if *exh {
            exhausted += 1;
        }
        if *w {
            wf += 1;
            if *n > 0 {
                wf_parsed += 1;
            }
        } else {
            nwf += 1;
            if *n > 0 {
                nwf_parsed += 1;
            }
        }
        if *has_gold {
            with_gold += 1;
            if *hit {
                gold_hit += 1;
            }
            if first.is_some() && first == g {
                top1 += 1;
            }
        }
        let mark = match (w, n, hit, has_gold) {
            (true, 0, _, _) => "MISS",
            (false, n, _, _) if *n > 0 => "OVER",
            (_, _, false, true) => "NOGOLD",
            _ => "ok",
        };
        if mark != "ok" && std::env::var("VERBOSE").is_ok() {
            println!(
                "{mark} {id} [{n} readings, {el:?}{}] {text}",
                if *exh { ", exhausted" } else { "" }
            );
            if mark == "NOGOLD" {
                println!("   gold: {}", g.clone().unwrap_or_default());
                println!("   ours: {}", first.clone().unwrap_or_default());
            }
        }
    }
    println!(
        "{dir}: MRS identical to gold for {}/{} gold trees",
        mrs_ok.load(std::sync::atomic::Ordering::Relaxed),
        mrs_checked.load(std::sync::atomic::Ordering::Relaxed)
    );
    println!(
        "{dir}: grammatical {wf_parsed}/{wf} parsed; ungrammatical {nwf_parsed}/{nwf} parsed; gold tree found {gold_hit}/{with_gold}, ranked first {top1}; {exhausted} hit limits; total {total:?}"
    );
}

fn erg_names(t: Sexp) -> Sexp {
    match t {
        Sexp::List(items) => Sexp::List(
            items
                .into_iter()
                .map(|x| match x {
                    Sexp::Atom(a) => Sexp::Atom(emdysi_parse::rank::erg_name(&a).to_string()),
                    l => erg_names(l),
                })
                .collect(),
        ),
        a => a,
    }
}
