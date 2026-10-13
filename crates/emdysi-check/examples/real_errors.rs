//! How many real errors the checker catches: the `core` pack (spelling,
//! named grammar errors, consistency) on paragraphs that writers later
//! corrected (`corpora/real-errors/*.tsv`, typo, spelling and grammar fixes
//! from documentation histories), by kind of error.
//!
//! `cargo run --release -p emdysi-check --example real_errors -- [OUT.tsv]`
//!
//! An error counts as caught when a diagnostic on the paragraph before the
//! fix overlaps the changed words (for an inserted word, the words around
//! it). The paragraph after the fix is checked too: a diagnostic on the
//! same place there is a false flag. Every pair is written to OUT.tsv
//! (default `real-errors.tsv`): kind, caught, flagged after the fix,
//! before, after, the diagnostics' rules and messages.
//!
//! With `EMDYSI_LM` set to a model (a spec as for `en --lm`), spelling and
//! grammar corrections are chosen by the model among those the grammar
//! verified (`emdysi_check::assist`); one model is opened per thread.
//! With `ORACLE` set instead, a perfect model answers: the version of a
//! sentence the writer's correction has (an upper bound for any model).
//! Either way the number of questions asked is reported. `SHARD=i/n`
//! checks only every n-th pair, starting with the i-th.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

use emdysi_check::*;
use emdysi_parse::{Erg, default_grammar_dir};

/// The changed span of `before` against `after`, in bytes, widened to whole
/// words (and to the words around an insertion).
fn changed_span(before: &str, after: &str) -> (usize, usize) {
    let b: Vec<char> = before.chars().collect();
    let a: Vec<char> = after.chars().collect();
    let mut p = 0;
    while p < b.len() && p < a.len() && b[p] == a[p] {
        p += 1;
    }
    let mut s = 0;
    while s < b.len() - p && s < a.len() - p && b[b.len() - 1 - s] == a[a.len() - 1 - s] {
        s += 1;
    }
    let (mut from, mut to) = (p, b.len() - s);
    while from > 0 && !b[from - 1].is_whitespace() {
        from -= 1;
    }
    while to < b.len() && !b[to].is_whitespace() {
        to += 1;
    }
    if from == to {
        // An insertion: the words on either side.
        while from > 0 && b[from - 1].is_whitespace() {
            from -= 1;
        }
        while from > 0 && !b[from - 1].is_whitespace() {
            from -= 1;
        }
        while to < b.len() && b[to].is_whitespace() {
            to += 1;
        }
        while to < b.len() && !b[to].is_whitespace() {
            to += 1;
        }
    }
    let byte = |c: usize| b[..c].iter().map(|x| x.len_utf8()).sum::<usize>();
    (byte(from), byte(to))
}

#[derive(Default)]
struct Tally {
    pairs: usize,
    caught: usize,
    after: usize,
}

fn main() {
    let out_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "real-errors.tsv".into());
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files: Vec<_> = std::fs::read_dir(root.join("corpora/real-errors"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "tsv"))
        .collect();
    files.sort();
    let rows: Vec<Vec<String>> = files
        .iter()
        .flat_map(|p| {
            std::fs::read_to_string(p)
                .unwrap()
                .lines()
                .filter(|l| !l.starts_with('#') && !l.is_empty())
                .map(|l| l.split('\t').map(String::from).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        })
        .filter(|r| r.len() >= 6)
        .collect();
    // SHARD=i/n: only every n-th pair, starting with the i-th (to split a
    // long run, e.g. with a model, into parts).
    let rows: Vec<Vec<String>> = match std::env::var("SHARD").ok().and_then(|s| {
        let (i, n) = s.split_once('/')?;
        Some((i.parse::<usize>().ok()?, n.parse::<usize>().ok()?))
    }) {
        Some((i, n)) if n > 0 => rows
            .into_iter()
            .enumerate()
            .filter(|(k, _)| k % n == i)
            .map(|(_, r)| r)
            .collect(),
        _ => rows,
    };
    let core =
        Pack::parse(&std::fs::read_to_string(root.join("packs/core.toml")).unwrap()).unwrap();
    let checker = Checker::new(vec![core]);
    let mut erg = Erg::load(&default_grammar_dir()).unwrap();
    // Parses are cached between runs (see `Erg::cache_parses`).
    if let Some(dir) = emdysi_parse::parse_cache_dir() {
        erg.cache_parses(&dir);
    }
    let opts = Options {
        threads: 1,
        ..Options::default()
    };
    let tallies: Mutex<BTreeMap<String, Tally>> = Mutex::default();
    let lines: Mutex<Vec<String>> = Mutex::default();
    let next = Mutex::new(0usize);
    std::thread::scope(|s| {
        let threads = std::env::var("THREADS")
            .ok()
            .and_then(|n| n.parse().ok())
            .unwrap_or_else(|| std::thread::available_parallelism().map_or(2, |n| n.get()));
        for _ in 0..threads {
            s.spawn(|| {
                let mut lm = model();
                let oracle_on = std::env::var_os("ORACLE").is_some();
                loop {
                    let i = {
                        let mut n = next.lock().unwrap();
                        *n += 1;
                        *n - 1
                    };
                    let Some(r) = rows.get(i) else { break };
                    let (kind, before, after) = (&r[1], &r[4], &r[5]);
                    let (from, to) = changed_span(before, after);
                    let (afrom, ato) = changed_span(after, before);
                    let mut oracle = Oracle {
                        truth: after.to_string(),
                    };
                    let mut check = |text: &str, from: usize, to: usize| -> Vec<Diagnostic> {
                        let a = analyze(&erg, text, Format::Markdown, &opts);
                        let ask: Option<&mut dyn emdysi_check::decisions::Ask> = if oracle_on {
                            Some(&mut oracle)
                        } else {
                            lm.as_mut()
                                .map(|m| m as &mut dyn emdysi_check::decisions::Ask)
                        };
                        checker
                            .check_with(&erg, &a, ask)
                            .into_iter()
                            .filter(|d| d.range.start < to && from < d.range.end)
                            .collect()
                    };
                    let hits = check(before, from, to);
                    let false_hits = check(after, afrom, ato);
                    let describe = |ds: &[Diagnostic]| {
                        ds.iter()
                            .map(|d| format!("{}: {}", d.rule, d.message))
                            .collect::<Vec<_>>()
                            .join(" | ")
                    };
                    lines.lock().unwrap().push(format!(
                        "{kind}\t{}\t{}\t{}\t{}\t{}\t{}",
                        !hits.is_empty(),
                        !false_hits.is_empty(),
                        &before[from..to],
                        &after[afrom..ato],
                        describe(&hits),
                        describe(&false_hits)
                    ));
                    let mut t = tallies.lock().unwrap();
                    let t = t.entry(kind.clone()).or_default();
                    t.pairs += 1;
                    t.caught += usize::from(!hits.is_empty());
                    t.after += usize::from(!false_hits.is_empty());
                }
            });
        }
    });
    let mut lines = lines.into_inner().unwrap();
    lines.sort();
    std::fs::write(
        &out_path,
        "# kind\tcaught\tflagged after fix\tbefore\tafter\tdiagnostics before\tdiagnostics after\n"
            .to_string()
            + &lines.join("\n")
            + "\n",
    )
    .unwrap();
    println!("| Kind | Pairs | Caught | Flagged after the fix |");
    println!("|---|---|---|---|");
    let mut all = Tally::default();
    for (k, t) in tallies.into_inner().unwrap() {
        println!("| {k} | {} | {} | {} |", t.pairs, t.caught, t.after);
        all.pairs += t.pairs;
        all.caught += t.caught;
        all.after += t.after;
    }
    println!(
        "| all | {} | {} ({:.0}%) | {} |",
        all.pairs,
        all.caught,
        100.0 * all.caught as f64 / all.pairs.max(1) as f64,
        all.after
    );
    let asked = QUESTIONS.load(std::sync::atomic::Ordering::Relaxed);
    if asked > 0 {
        println!("\nQuestions asked of the model: {asked}.");
    }
}

/// The model named by `EMDYSI_LM` (a spec as for `en --lm`), if any, as the
/// checker's decision model: spelling and grammar corrections are then
/// chosen by it among those the grammar verified.
struct ModelAsk(Box<dyn emdysi_lm::LanguageModel>);

impl emdysi_check::decisions::Ask for ModelAsk {
    fn ask(&mut self, context: &str, question: &str, options: &[&str]) -> Option<Vec<f64>> {
        QUESTIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        emdysi_lm::decide::Decider::new(&mut *self.0)
            .choose(context, question, options)
            .ok()
    }
}

fn model() -> Option<ModelAsk> {
    let spec = std::env::var("EMDYSI_LM")
        .ok()
        .filter(|s| !s.trim().is_empty())?;
    Some(ModelAsk(emdysi_lm::open(spec.trim()).expect("EMDYSI_LM")))
}

/// Questions asked of the model (or the oracle), for the report.
static QUESTIONS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// With `ORACLE` set, a perfect decision model: it chooses the version of
/// a sentence that the writer's own correction has, or else the original.
/// Recall with it is an upper bound for any model on the corrections the
/// grammar verified.
struct Oracle {
    truth: String,
}

impl emdysi_check::decisions::Ask for Oracle {
    fn ask(&mut self, _context: &str, question: &str, options: &[&str]) -> Option<Vec<f64>> {
        QUESTIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // Screening: the sentence is correct if the corrected text has it.
        if let Some(sentence) = question
            .strip_prefix(emdysi_check::assist::SCREEN_QUESTION)
            .and_then(|q| q.split("Sentence: ").nth(1))
        {
            let ok = self.truth.contains(sentence.trim());
            return Some(if ok { vec![1.0, 0.0] } else { vec![0.0, 1.0] });
        }
        let right = options
            .iter()
            .position(|o| self.truth.contains(o.trim()))
            .unwrap_or(0);
        Some(
            (0..options.len())
                .map(|i| if i == right { 1.0 } else { 0.0 })
                .collect(),
        )
    }
}
