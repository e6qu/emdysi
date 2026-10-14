//! Why a perfect decision model misses a real grammar error: for each
//! grammar pair of `corpora/real-errors/*.tsv`, the first stage of
//! `emdysi_check::assist` that loses the writer's correction.
//!
//! `cargo run --release -p emdysi-check --example fix_bound -- [OUT.tsv]`
//!
//! Stages, in order: the change spans more than one sentence; the sentence
//! is not parsed (or is too short or long); the correction is not among
//! the candidate edits; it is past the candidates tried
//! (`MAX_CANDIDATES`); the strict grammar rejects the corrected sentence
//! (coverage); it is past the versions shown to the model
//! (`MAX_OPTIONS`). A pair that passes them all is reachable (a sentence
//! the grammar accepts needs screening as well). No model is needed.
//! Every pair is written to OUT.tsv (default `fix-bound.tsv`): kind,
//! stage, the candidate's index, sentence before, sentence after.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

use emdysi_check::assist::{MAX_CANDIDATES, MAX_OPTIONS, edits, strict};
use emdysi_check::*;
use emdysi_parse::{Erg, default_grammar_dir};

/// The changed characters of `a` against `b`: (from, to in `a`, to in `b`).
fn diff(a: &[char], b: &[char]) -> (usize, usize, usize) {
    let mut p = 0;
    while p < a.len() && p < b.len() && a[p] == b[p] {
        p += 1;
    }
    let mut s = 0;
    while s < a.len() - p && s < b.len() - p && a[a.len() - 1 - s] == b[b.len() - 1 - s] {
        s += 1;
    }
    (p, a.len() - s, b.len() - s)
}

fn stage(erg: &Erg, opts: &Options, before: &str, after: &str) -> (String, String, String, String) {
    let none = |st: &str| (st.to_string(), String::new(), String::new(), String::new());
    let (b, a): (Vec<char>, Vec<char>) = (before.chars().collect(), after.chars().collect());
    let (from, bto, ato) = diff(&b, &a);
    let analysis = analyze(erg, before, Format::Markdown, opts);
    // The sentence holding the change, by its position in the paragraph.
    let mut found = None;
    for s in &analysis.sentences {
        let text = s.original.trim();
        let Some(byte) = before.find(text) else {
            continue;
        };
        let start = before[..byte].chars().count();
        let end = start + text.chars().count();
        if start <= from && bto <= end {
            found = Some((s, start, end));
            break;
        }
    }
    let Some((s, start, end)) = found else {
        return none("spans sentences");
    };
    let sb: String = b[start..end].iter().collect();
    let sa: String = b[start..from]
        .iter()
        .chain(&a[from..ato])
        .chain(&b[bto..end])
        .collect();
    let row = |st: &str, i: String| (st.to_string(), i, sb.clone(), sa.clone());
    if s.parse.is_none() || !(3..=40).contains(&s.word_count()) {
        return row("not parsed", String::new());
    }
    let es = edits(erg, s);
    let Some(i) = es
        .iter()
        .position(|e| e.apply(&s.original).trim() == sa.trim())
    else {
        return row("not a candidate", String::new());
    };
    if i >= MAX_CANDIDATES {
        return row("past the candidates tried", i.to_string());
    }
    if !strict(erg, &sa) {
        return row("corrected sentence not strict", i.to_string());
    }
    let shown = es[..i]
        .iter()
        .filter(|e| strict(erg, &e.apply(&s.original)))
        .count();
    if shown >= MAX_OPTIONS {
        return row("past the versions shown", format!("{i}/{shown}"));
    }
    if s.strict() {
        return row("reachable with screening", format!("{i}/{shown}"));
    }
    row("reachable", format!("{i}/{shown}"))
}

fn main() {
    let out_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "fix-bound.tsv".into());
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut rows = Vec::new();
    for e in std::fs::read_dir(root.join("corpora/real-errors")).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_none_or(|x| x != "tsv") {
            continue;
        }
        for l in std::fs::read_to_string(&p).unwrap().lines() {
            let r: Vec<String> = l.split('\t').map(String::from).collect();
            if !l.starts_with('#') && r.len() >= 6 && r[1].starts_with("grammar") {
                rows.push(r);
            }
        }
    }
    let mut erg = Erg::load(&default_grammar_dir()).unwrap();
    if let Some(dir) = emdysi_parse::parse_cache_dir() {
        erg.cache_parses(&dir);
    }
    let opts = Options {
        threads: 1,
        ..Options::default()
    };
    let lines: Mutex<Vec<String>> = Mutex::default();
    let tally: Mutex<BTreeMap<(String, String), usize>> = Mutex::default();
    let next = Mutex::new(0usize);
    std::thread::scope(|sc| {
        let threads = std::env::var("THREADS")
            .ok()
            .and_then(|n| n.parse().ok())
            .unwrap_or_else(|| std::thread::available_parallelism().map_or(2, |n| n.get()));
        for _ in 0..threads {
            sc.spawn(|| {
                loop {
                    let i = {
                        let mut n = next.lock().unwrap();
                        *n += 1;
                        *n - 1
                    };
                    let Some(r) = rows.get(i) else { break };
                    let (st, idx, sb, sa) = stage(&erg, &opts, &r[4], &r[5]);
                    lines
                        .lock()
                        .unwrap()
                        .push(format!("{}\t{st}\t{idx}\t{sb}\t{sa}", r[1]));
                    *tally.lock().unwrap().entry((st, r[1].clone())).or_default() += 1;
                }
            });
        }
    });
    let mut lines = lines.into_inner().unwrap();
    lines.sort();
    std::fs::write(
        &out_path,
        "# kind\tstage\tcandidate (index/verified before it)\tsentence before\tsentence after\n"
            .to_string()
            + &lines.join("\n")
            + "\n",
    )
    .unwrap();
    println!("| Stage | Kind | Pairs |\n|---|---|---|");
    for ((st, k), n) in tally.into_inner().unwrap() {
        println!("| {st} | {k} | {n} |");
    }
}
