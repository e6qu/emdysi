//! Evaluate against [incr tsdb()] gold profiles:
//! `cargo run --release --example eval -- corpora/erg-gold/mrs [limit]`.
//!
//! Reports coverage on grammatical items, rejection of ungrammatical ones,
//! and how often the gold derivation is among the readings.

use std::collections::HashMap;
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

use emdysi_parse::*;

fn read_relation(dir: &str, name: &str) -> Vec<Vec<String>> {
    let path = format!("{dir}/{name}.gz");
    let out = Command::new("zcat").arg(&path).output().expect("zcat");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.split('@').map(unescape).collect())
        .collect()
}

/// Undo [incr tsdb()] field escaping (`\\`, `\s` for `@`, `\n`).
fn unescape(f: &str) -> String {
    let mut out = String::new();
    let mut chars = f.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('s') => out.push('@'),
                Some('n') => out.push('\n'),
                Some(d) => out.push(d),
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[derive(Debug)]
enum Sexp {
    Atom(String),
    List(Vec<Sexp>),
}

fn parse_sexp(s: &str) -> Option<Sexp> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    parse_at(&chars, &mut i)
}

fn parse_at(c: &[char], i: &mut usize) -> Option<Sexp> {
    while *i < c.len() && c[*i].is_whitespace() {
        *i += 1;
    }
    if *i >= c.len() {
        return None;
    }
    match c[*i] {
        '(' => {
            *i += 1;
            let mut items = Vec::new();
            loop {
                while *i < c.len() && c[*i].is_whitespace() {
                    *i += 1;
                }
                if *i >= c.len() {
                    return None;
                }
                if c[*i] == ')' {
                    *i += 1;
                    return Some(Sexp::List(items));
                }
                items.push(parse_at(c, i)?);
            }
        }
        '"' => {
            *i += 1;
            let mut s = String::new();
            while *i < c.len() && c[*i] != '"' {
                if c[*i] == '\\' {
                    *i += 1;
                }
                if *i < c.len() {
                    s.push(c[*i]);
                }
                *i += 1;
            }
            *i += 1;
            Some(Sexp::Atom(format!("\"{s}\"")))
        }
        _ => {
            let mut s = String::new();
            while *i < c.len() && !c[*i].is_whitespace() && c[*i] != '(' && c[*i] != ')' {
                s.push(c[*i]);
                *i += 1;
            }
            Some(Sexp::Atom(s))
        }
    }
}

/// Canonical skeleton `(name start end children...)`, dropping leaves.
fn skeleton(s: &Sexp, gold: bool) -> String {
    let Sexp::List(items) = s else {
        return String::new();
    };
    let atoms: Vec<&str> = items
        .iter()
        .map_while(|x| match x {
            Sexp::Atom(a) => Some(a.as_str()),
            _ => None,
        })
        .collect();
    // Gold: (id name score start end kids) or (root kid); ours: (name start end kids).
    let (name, span) = if gold {
        match atoms.len() {
            1 => (atoms[0], None),
            n if n >= 5 => (atoms[1], Some((atoms[3], atoms[4]))),
            _ => return String::new(),
        }
    } else {
        match atoms.len() {
            n if n >= 3 => (atoms[0], Some((atoms[1], atoms[2]))),
            _ => return String::new(),
        }
    };
    if name.starts_with('"') {
        return String::new();
    }
    let kids: Vec<String> = items[atoms.len()..]
        .iter()
        .map(|k| skeleton(k, gold))
        .filter(|k| !k.is_empty())
        .collect();
    match span {
        Some((a, b)) => format!(
            "({name} {a} {b}{}{})",
            if kids.is_empty() { "" } else { " " },
            kids.join(" ")
        ),
        None => kids.join(" "),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = &args[1];
    let limit: usize = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(usize::MAX);
    let mut erg = Erg::load(&default_grammar_dir()).unwrap();
    erg.config.timeout = Duration::from_secs(20);
    erg.config.max_edges = 60_000;

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
    for f in read_relation(dir, "result") {
        if f.len() > 10 {
            if let (Some(item), Some(tree)) = (parse_to_item.get(&f[0]), parse_sexp(&f[10])) {
                gold.insert(item.clone(), skeleton(&tree, true));
            }
        }
    }

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
                    let p = erg.parse(text).unwrap();
                    let ours: Vec<String> = p
                        .readings
                        .iter()
                        .filter_map(|r| parse_sexp(&r.derivation).map(|t| skeleton(&t, false)))
                        .collect();
                    let g = gold.get(id).cloned();
                    let hit = g.as_ref().is_some_and(|g| ours.iter().any(|o| o == g));
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
    let (
        mut wf,
        mut wf_parsed,
        mut nwf,
        mut nwf_parsed,
        mut with_gold,
        mut gold_hit,
        mut exhausted,
    ) = (0, 0, 0, 0, 0, 0, 0);
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
        "{dir}: grammatical {wf_parsed}/{wf} parsed; ungrammatical {nwf_parsed}/{nwf} parsed; gold tree found {gold_hit}/{with_gold}; {exhausted} hit limits; total {total:?}"
    );
}
