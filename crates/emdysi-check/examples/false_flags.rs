//! How often the checker claims an error in edited text, per genre: the
//! false-flag rate of the `core` pack (spelling, grammar, consistency) on
//! the samples in `corpora/edited/` and `corpora/edited-by-sa/`, per 1,000
//! sentences: the first development set by default; `SET=heldout` for the
//! held-out test set (`heldout-*.tsv`), `SET=dev2` for the second
//! development set only. The target is at most
//! one per 1,000 (99.9% precision on clean text).
//!
//! `cargo run --release -p emdysi-check --example false_flags -- [OUT.tsv] [GENRE,...]`
//!
//! A sample row may list annotated errors ("form>correction"); a flag
//! covering one counts as a catch, not a false flag, and the catches are
//! reported as recall (none of the current sources has any). Every
//! flag is written to OUT.tsv (default `false-flags.tsv` in the current
//! directory) for inspection: genre, document, rule, flagged text, message,
//! sentence. Finished documents are also appended to OUT.tsv.journal, so
//! that a rerun with the same OUT resumes an interrupted run; `THREADS`
//! caps the documents checked at once.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

use emdysi_check::*;
use emdysi_parse::{Erg, default_grammar_dir};

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(o) => out.push(o),
            None => out.push('\\'),
        }
    }
    out
}

#[derive(Default)]
struct Tally {
    documents: usize,
    sentences: usize,
    flags: BTreeMap<String, usize>,
    annotated: usize,
    caught: usize,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out_path = args
        .first()
        .cloned()
        .unwrap_or_else(|| "false-flags.tsv".into());
    let only: Option<Vec<String>> = args
        .get(1)
        .map(|g| g.split(',').map(String::from).collect());
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // The development set by default; `SET=heldout` measures the held-out
    // test set instead (heldout-*.tsv), which is not used while
    // developing the checks.
    let set = std::env::var("SET").ok();
    let mut files: Vec<std::path::PathBuf> = ["corpora/edited", "corpora/edited-by-sa"]
        .iter()
        .flat_map(|d| std::fs::read_dir(root.join(d)).unwrap())
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "tsv"))
        .filter(|p| {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            match set.as_deref() {
                Some(prefix) => name.starts_with(&format!("{prefix}-")),
                // The first development set: files without a set prefix.
                None => !name.starts_with("heldout-") && !name.starts_with("dev2-"),
            }
        })
        .collect();
    files.sort();
    let src = files
        .iter()
        .map(|p| std::fs::read_to_string(p).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    let rows: Vec<Vec<String>> = src
        .lines()
        .filter(|l| !l.is_empty())
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split('\t').map(unescape).collect())
        .filter(|r: &Vec<String>| only.as_ref().is_none_or(|g| g.contains(&r[1])))
        .collect();
    let core =
        Pack::parse(&std::fs::read_to_string(root.join("packs/core.toml")).unwrap()).unwrap();
    let checker = Checker::new(vec![core]);
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let opts = Options {
        threads: 1,
        ..Options::default()
    };
    let tallies: Mutex<BTreeMap<String, Tally>> = Mutex::default();
    let lines: Mutex<Vec<String>> = Mutex::default();
    // Each finished document is appended to OUT.tsv.journal, and a rerun
    // with the same OUT skips the documents already there: a long run that
    // is interrupted resumes where it stopped.
    let journal_path = format!("{out_path}.journal");
    let mut done: std::collections::HashSet<String> = Default::default();
    if let Ok(j) = std::fs::read_to_string(&journal_path) {
        let mut t = tallies.lock().unwrap();
        let mut l = lines.lock().unwrap();
        for line in j.lines() {
            let f: Vec<&str> = line.splitn(2, '\t').collect();
            match f.as_slice() {
                ["D", rest] => {
                    let g: Vec<&str> = rest.split('\t').collect();
                    if let [genre, id, sentences, annotated, caught, rules] = g[..] {
                        let e = t.entry(genre.to_string()).or_default();
                        e.documents += 1;
                        e.sentences += sentences.parse::<usize>().unwrap_or(0);
                        e.annotated += annotated.parse::<usize>().unwrap_or(0);
                        e.caught += caught.parse::<usize>().unwrap_or(0);
                        for r in rules.split(',').filter(|r| !r.is_empty()) {
                            *e.flags.entry(r.to_string()).or_default() += 1;
                        }
                        done.insert(id.to_string());
                    }
                }
                ["F", rest] => l.push(rest.to_string()),
                _ => {}
            }
        }
        eprintln!("resuming: {} documents already done", done.len());
    }
    let journal = Mutex::new(
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&journal_path)
            .unwrap(),
    );
    let rows: Vec<&Vec<String>> = rows.iter().filter(|r| !done.contains(&r[0])).collect();
    let next = Mutex::new(0usize);
    let t0 = std::time::Instant::now();
    std::thread::scope(|s| {
        // THREADS caps the documents checked at once (and so the memory).
        let threads = std::env::var("THREADS")
            .ok()
            .and_then(|n| n.parse().ok())
            .unwrap_or_else(|| std::thread::available_parallelism().map_or(2, |n| n.get()));
        for _ in 0..threads {
            s.spawn(|| {
                loop {
                    let i = {
                        let mut n = next.lock().unwrap();
                        *n += 1;
                        *n - 1
                    };
                    let Some(r) = rows.get(i) else { break };
                    let (id, genre, format, text) = (&r[0], &r[1], &r[3], &r[4]);
                    let annotated: Vec<&str> = r
                        .get(5)
                        .map(|e| e.split(" | ").filter(|x| !x.is_empty()).collect())
                        .unwrap_or_default();
                    let format = if format == "md" {
                        Format::Markdown
                    } else {
                        Format::Plain
                    };
                    let a = analyze(&erg, text, format, &opts);
                    let diags = checker.check(&erg, &a);
                    let mut caught = 0;
                    let mut flags: Vec<String> = Vec::new();
                    let mut out = Vec::new();
                    for d in &diags {
                        let flagged = text.get(d.range.clone()).unwrap_or("");
                        let hit = annotated.iter().any(|e| {
                            let form = e.split('>').next().unwrap_or("");
                            !form.is_empty() && flagged.contains(form)
                        });
                        let sentence = d
                            .sentence
                            .and_then(|k| a.sentences.get(k))
                            .map_or(String::new(), |s| {
                                s.original.trim().replace(['\t', '\n'], " ")
                            });
                        if hit {
                            caught += 1;
                        } else {
                            flags.push(d.rule.clone());
                        }
                        out.push(format!(
                            "{genre}\t{id}\t{}\t{}\t{}\t{}",
                            if hit {
                                format!("{} (annotated)", d.rule)
                            } else {
                                d.rule.clone()
                            },
                            flagged.replace(['\t', '\n'], " "),
                            d.message.replace(['\t', '\n'], " "),
                            sentence
                        ));
                    }
                    {
                        use std::io::Write;
                        let mut entry = String::new();
                        for o in &out {
                            entry.push_str(&format!("F\t{o}\n"));
                        }
                        entry.push_str(&format!(
                            "D\t{genre}\t{id}\t{}\t{}\t{}\t{}\n",
                            a.sentences.len(),
                            annotated.len(),
                            caught.min(annotated.len()),
                            flags.join(",")
                        ));
                        let mut j = journal.lock().unwrap();
                        j.write_all(entry.as_bytes()).unwrap();
                        j.flush().unwrap();
                    }
                    lines.lock().unwrap().extend(out);
                    let mut t = tallies.lock().unwrap();
                    let t = t.entry(genre.clone()).or_default();
                    t.documents += 1;
                    t.sentences += a.sentences.len();
                    t.annotated += annotated.len();
                    t.caught += caught.min(annotated.len());
                    for f in flags {
                        *t.flags.entry(f).or_default() += 1;
                    }
                }
            });
        }
    });
    let mut lines = lines.into_inner().unwrap();
    lines.sort();
    std::fs::write(
        &out_path,
        "# genre\tdocument\trule\tflagged\tmessage\tsentence\n".to_string()
            + &lines.join("\n")
            + "\n",
    )
    .unwrap();
    let tallies = tallies.into_inner().unwrap();
    let rules: Vec<String> = {
        let mut r: Vec<String> = tallies
            .values()
            .flat_map(|t| t.flags.keys().cloned())
            .collect();
        r.sort();
        r.dedup();
        r
    };
    println!(
        "| Genre | Documents | Sentences | Flags per 1,000 sentences | {} | Annotated errors caught |",
        rules.join(" | ")
    );
    println!("|---|---|---|---|{}---|", "---|".repeat(rules.len()));
    let mut all = Tally::default();
    for (g, t) in &tallies {
        let total: usize = t.flags.values().sum();
        let per = |n: usize| 1000.0 * n as f64 / t.sentences.max(1) as f64;
        let cells: Vec<String> = rules
            .iter()
            .map(|r| format!("{:.1}", per(t.flags.get(r).copied().unwrap_or(0))))
            .collect();
        let caught = if t.annotated > 0 {
            format!("{}/{}", t.caught, t.annotated)
        } else {
            "-".into()
        };
        println!(
            "| {g} | {} | {} | {:.1} | {} | {caught} |",
            t.documents,
            t.sentences,
            per(total),
            cells.join(" | ")
        );
        all.documents += t.documents;
        all.sentences += t.sentences;
        for (r, n) in &t.flags {
            *all.flags.entry(r.clone()).or_default() += n;
        }
    }
    let total: usize = all.flags.values().sum();
    println!(
        "\nAll: {} sentences, {} flags, {:.1} per 1,000 ({:.2}% of sentences flagged at most); {:.0}s. Flags written to {out_path}.",
        all.sentences,
        total,
        1000.0 * total as f64 / all.sentences.max(1) as f64,
        100.0 * total as f64 / all.sentences.max(1) as f64,
        t0.elapsed().as_secs_f64()
    );
}
