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
//! before, after, the diagnostics' rules and messages. Each finished pair
//! is also appended to OUT.tsv.journal, and a rerun with the same OUT
//! (and the same SHARD and ONLY) resumes after the pairs there; progress
//! goes to standard error every ten pairs.
//!
//! With `EMDYSI_LM` set to a model (a spec as for `en --lm`), spelling and
//! grammar corrections are chosen by the model among those the grammar
//! verified (`emdysi_check::assist`); one model is opened per thread.
//! With `ORACLE` set instead, a perfect model answers: the version of a
//! sentence the writer's correction has (an upper bound for any model).
//! Either way the number of questions asked is reported. `SHARD=i/n`
//! checks only every n-th pair, starting with the i-th; `PAIRS=i,j,...`
//! only those pairs, by position after SHARD (as the journal numbers them).
//! `LOG=FILE` with a model appends each question it is asked to FILE, with
//! its probabilities and the answer a perfect model gives: how well the
//! model screens, locates and chooses, and the probabilities to tune the
//! thresholds on; the questions that choose among versions of a sentence
//! also go to FILE.versions.tsv, as `en decide --eval` reads them, to
//! score again without parsing. `LETTERS` asks for a letter even where a model that
//! scores text would compare the versions of a sentence by likelihood.
//! `PACKS=packs/a.toml,...` adds packs to `core`; `CORE=FILE` replaces
//! `core` itself (another threshold, say). `EXPLAIN=FILE` writes
//! the grammar's account of each missed grammar error (see the `why`
//! example).

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
    // ONLY=a|b: only the pairs whose text before the correction is one of
    // these, or whose inserted text is when nothing is removed (to look at a
    // few pairs).
    let rows: Vec<Vec<String>> = match std::env::var("ONLY") {
        Ok(only) => {
            let only: Vec<&str> = only.split('|').collect();
            rows.into_iter()
                .filter(|r| {
                    let key = if r[2].is_empty() { &r[3] } else { &r[2] };
                    only.contains(&key.as_str())
                })
                .collect()
        }
        Err(_) => rows,
    };
    // PAIRS=i,j,...: only these pairs, by their position after SHARD and
    // ONLY (as the journal numbers them), e.g. those a perfect model
    // catches and the grammar alone does not.
    let rows: Vec<Vec<String>> = match std::env::var("PAIRS") {
        Ok(pairs) => {
            let keep: Vec<usize> = pairs
                .split(',')
                .filter_map(|i| i.trim().parse().ok())
                .collect();
            rows.into_iter()
                .enumerate()
                .filter(|(k, _)| keep.contains(k))
                .map(|(_, r)| r)
                .collect()
        }
        Err(_) => rows,
    };
    let core =
        // `CORE=FILE`: another version of the `core` pack, e.g. with other
        // thresholds.
        Pack::parse(&std::fs::read_to_string(root.join(std::env::var("CORE").unwrap_or_else(|_| "packs/core.toml".into()))).unwrap()).unwrap();
    // `PACKS=packs/a.toml,packs/b.toml` adds packs to `core`.
    let mut packs = vec![core];
    for p in std::env::var("PACKS")
        .unwrap_or_default()
        .split(',')
        .filter(|p| !p.is_empty())
    {
        packs.push(Pack::parse(&std::fs::read_to_string(root.join(p)).unwrap()).unwrap());
    }
    let checker = Checker::new(packs);
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
    // Each finished pair is appended to OUT.tsv.journal as its index and
    // its line; a rerun with the same OUT (and the same SHARD and ONLY)
    // resumes after the pairs there.
    let journal = Appender::open(&format!("{out_path}.journal"), "");
    let done: BTreeMap<usize, String> = std::fs::read_to_string(format!("{out_path}.journal"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let (i, line) = l.split_once('\t')?;
            Some((i.parse().ok()?, line.to_string()))
        })
        .collect();
    for line in done.values() {
        let f: Vec<&str> = line.split('\t').collect();
        let mut t = tallies.lock().unwrap();
        let t = t.entry(f[0].to_string()).or_default();
        t.pairs += 1;
        t.caught += usize::from(f[1] == "true");
        t.after += usize::from(f[2] == "true");
        lines.lock().unwrap().push(line.clone());
    }
    // `EXPLAIN=FILE`: the grammar's account of each missed grammar error.
    let explain_path = std::env::var("EXPLAIN").ok();
    let explain: Option<Mutex<Vec<String>>> = explain_path.as_ref().map(|_| Mutex::default());
    // `LOG=FILE`: each question asked of the model, with the perfect answer.
    // Appended as the questions are asked.
    let log_path = std::env::var("LOG").ok();
    let log = log_path.as_ref().map(|p| {
        (
            Appender::open(
                p,
                "# pair\tkind\tside\tquestion\toptions\tright\tchosen\tp(right)\tp(original)\tprobabilities",
            ),
            Appender::open(&format!("{p}.versions.tsv"), ""),
        )
    });
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
                let truth_on = oracle_on || log_path.is_some();
                loop {
                    let i = {
                        let mut n = next.lock().unwrap();
                        *n += 1;
                        *n - 1
                    };
                    let Some(r) = rows.get(i) else { break };
                    if done.contains_key(&i) {
                        continue;
                    }
                    let (kind, before, after) = (&r[1], &r[4], &r[5]);
                    let (from, to) = changed_span(before, after);
                    let (afrom, ato) = changed_span(after, before);
                    // The corrected sentences as the checker sees them (the
                    // rendered Markdown, without link targets or code marks).
                    let mut oracle = Oracle {
                        truth: if truth_on {
                            analyze(&erg, after, Format::Markdown, &opts)
                                .sentences
                                .iter()
                                .map(|s| s.original.trim().to_string())
                                .collect::<Vec<_>>()
                                .join("\n")
                        } else {
                            String::new()
                        },
                    };
                    let mut check =
                        |text: &str, from: usize, to: usize, side: &str| -> Vec<Diagnostic> {
                            let a = analyze(&erg, text, Format::Markdown, &opts);
                            let mut logged = lm.as_mut().zip(log.as_ref()).map(|(m, log)| Logged {
                                inner: m,
                                oracle: &oracle,
                                log: &log.0,
                                versions: &log.1,
                                tag: format!("{i}\t{kind}\t{side}"),
                            });
                            let ask: Option<&mut dyn emdysi_check::decisions::Ask> = if oracle_on {
                                Some(&mut oracle)
                            } else if let Some(l) = logged.as_mut() {
                                Some(l)
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
                    let hits = check(before, from, to, "before");
                    let false_hits = check(after, afrom, ato, "after");
                    // Why a grammar error was missed: the grammar's account
                    // of each sentence the change touches.
                    if hits.is_empty() && kind.starts_with("grammar") {
                        if let Some(out) = &explain {
                            let a = analyze(&erg, before, Format::Markdown, &opts);
                            for si in 0..a.sentences.len() {
                                let r = a.sentence_source(si);
                                if r.start < to && from < r.end {
                                    out.lock().unwrap().push(format!(
                                        "## {kind}: {:?} -> {:?}\n{}\n{}\n",
                                        &before[from..to],
                                        &after[afrom..ato],
                                        a.sentences[si].original,
                                        emdysi_check::rules::explain_grammar(&erg, &a, si)
                                    ));
                                }
                            }
                        }
                    }
                    let describe = |ds: &[Diagnostic]| {
                        ds.iter()
                            .map(|d| format!("{}: {}", d.rule, d.message))
                            .collect::<Vec<_>>()
                            .join(" | ")
                    };
                    let line = format!(
                        "{kind}\t{}\t{}\t{}\t{}\t{}\t{}",
                        !hits.is_empty(),
                        !false_hits.is_empty(),
                        &before[from..to],
                        &after[afrom..ato],
                        describe(&hits),
                        describe(&false_hits)
                    );
                    journal.push(format!("{i}\t{line}"));
                    lines.lock().unwrap().push(line);
                    let mut t = tallies.lock().unwrap();
                    let t = t.entry(kind.clone()).or_default();
                    t.pairs += 1;
                    t.caught += usize::from(!hits.is_empty());
                    t.after += usize::from(!false_hits.is_empty());
                    let done = lines.lock().unwrap().len();
                    if done % 10 == 0 {
                        eprintln!("{done} of {} pairs", rows.len());
                    }
                }
            });
        }
    });
    if let (Some(path), Some(out)) = (&explain_path, explain) {
        let mut out = out.into_inner().unwrap();
        out.sort();
        std::fs::write(path, out.join("\n")).unwrap();
    }
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

    fn versions(&mut self, context: &str, question: &str, versions: &[&str]) -> Option<Vec<f64>> {
        // LETTERS: ask for a letter even when the model scores text.
        if std::env::var_os("LETTERS").is_some() {
            return self.ask(context, question, versions);
        }
        QUESTIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        emdysi_lm::decide::Decider::new(&mut *self.0)
            .versions(context, question, versions)
            .ok()
    }

    fn surprise(
        &mut self,
        context: &str,
        text: &str,
        spans: &[(usize, usize)],
    ) -> Option<Vec<f64>> {
        // LETTERS: the screening and locating questions instead.
        if std::env::var_os("LETTERS").is_some() {
            return None;
        }
        QUESTIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        emdysi_lm::decide::Decider::new(&mut *self.0)
            .surprise(context, text, spans)
            .ok()
    }

    fn estimates(&mut self, context: &str, versions: &[&str]) -> Option<Vec<f64>> {
        if std::env::var_os("LETTERS").is_some() {
            return None;
        }
        emdysi_lm::decide::Decider::new(&mut *self.0)
            .estimates(context, versions)
            .ok()
    }

    fn can_estimate(&self) -> bool {
        std::env::var_os("LETTERS").is_none() && self.0.scores_tokens()
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
/// a sentence that the writer's own correction has (one of the corrected
/// paragraph's sentences), or else the original.
/// Recall with it is an upper bound for any model on the corrections the
/// grammar verified.
struct Oracle {
    truth: String,
}

impl emdysi_check::decisions::Ask for Oracle {
    fn ask(&mut self, _context: &str, question: &str, options: &[&str]) -> Option<Vec<f64>> {
        QUESTIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Some(self.answer(question, options))
    }
}

impl Oracle {
    fn answer(&self, question: &str, options: &[&str]) -> Vec<f64> {
        if let Some(sentence) = question
            .strip_prefix(emdysi_check::assist::LOCATE_QUESTION)
            .and_then(|q| q.split("Sentence: ").nth(1))
        {
            return locate(sentence.trim(), &self.truth, options);
        }
        // Screening: the sentence is correct if the corrected text has it.
        if let Some(sentence) = question
            .strip_prefix(emdysi_check::assist::SCREEN_QUESTION)
            .and_then(|q| q.split("Sentence: ").nth(1))
        {
            let ok = self.truth.contains(sentence.trim());
            return if ok { vec![1.0, 0.0] } else { vec![0.0, 1.0] };
        }
        // An option that is a whole corrected sentence, else one inside
        // one: |will generate ...| is inside |This will generate ...|.
        let right = options
            .iter()
            .position(|o| self.truth.lines().any(|l| l == o.trim()))
            .or_else(|| options.iter().position(|o| self.truth.contains(o.trim())))
            .unwrap_or(0);
        if std::env::var_os("SPELLDBG").is_some() {
            eprintln!("Q {question:?}\n  options {options:?}\n  right {right}");
        }
        (0..options.len())
            .map(|i| if i == right { 1.0 } else { 0.0 })
            .collect()
    }
}

/// The model, with each question it answers logged next to the perfect
/// answer (`LOG`): pair, kind of error, side (the text before or after the
/// fix), kind of question, number of options, the perfect answer, the
/// model's choice, its probabilities for both and for the first option
/// (the original, in a grammar question), and all its probabilities.
struct Logged<'a> {
    inner: &'a mut ModelAsk,
    oracle: &'a Oracle,
    log: &'a Appender,
    /// The versions questions, as `en decide --eval` reads them.
    versions: &'a Appender,
    tag: String,
}

impl emdysi_check::decisions::Ask for Logged<'_> {
    fn ask(&mut self, context: &str, question: &str, options: &[&str]) -> Option<Vec<f64>> {
        let p = self.inner.ask(context, question, options)?;
        self.record(question, options, &p);
        Some(p)
    }

    fn versions(&mut self, context: &str, question: &str, versions: &[&str]) -> Option<Vec<f64>> {
        let p = self.inner.versions(context, question, versions)?;
        self.record(question, versions, &p);
        let flat = |t: &str| t.replace(['\t', '\n', '|'], " ");
        let right = self
            .oracle
            .answer(question, versions)
            .iter()
            .position(|&x| x > 0.5)
            .unwrap_or(0);
        self.versions.push(format!(
            "{}\t{}\t{}\t{right}",
            flat(question),
            flat(context),
            versions
                .iter()
                .map(|v| flat(v))
                .collect::<Vec<_>>()
                .join("|")
        ));
        Some(p)
    }

    /// Logged as a question of kind `surprise-wrong` or `surprise-right`
    /// (the perfect screening answer) whose options are the words, the
    /// right one the word a perfect model names as wrong; the
    /// probabilities are the words' surprise, and `p(original)` the
    /// highest.
    fn surprise(
        &mut self,
        context: &str,
        text: &str,
        spans: &[(usize, usize)],
    ) -> Option<Vec<f64>> {
        use emdysi_check::assist::*;
        let v = self.inner.surprise(context, text, spans)?;
        // The words labelled as `assist::word_options` labels them.
        let mut labels: Vec<String> = Vec::new();
        for &(f, t) in spans {
            let core = text[f..t].trim_matches(|c: char| !c.is_alphanumeric());
            let seen = labels
                .iter()
                .filter(|l| l.split(" (").next() == Some(core))
                .count();
            labels.push(if seen == 0 {
                core.to_string()
            } else {
                format!("{core} ({})", seen + 1)
            });
        }
        let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
        let sentence = text.trim();
        let wrong = self.oracle.answer(
            &format!("{SCREEN_QUESTION}\n\nSentence: {sentence}"),
            &["yes", "no"],
        )[1] > 0.5;
        let right = self
            .oracle
            .answer(
                &format!("{LOCATE_QUESTION}\n\nSentence: {sentence}"),
                &labels,
            )
            .iter()
            .position(|&p| p > 0.5)
            .unwrap_or(0);
        let argmax = (0..v.len())
            .max_by(|&a, &b| v[a].total_cmp(&v[b]))
            .unwrap_or(0);
        let peak = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let all = v
            .iter()
            .map(|x| format!("{x:.2}"))
            .collect::<Vec<_>>()
            .join(",");
        let kind = if wrong {
            "surprise-wrong"
        } else {
            "surprise-right"
        };
        self.log.push(format!(
            "{}\t{kind}\t{}\t{right}\t{argmax}\t{:.2}\t{peak:.2}\t{all}",
            self.tag,
            v.len(),
            v.get(right).copied().unwrap_or(0.0),
        ));
        Some(v)
    }

    fn estimates(&mut self, context: &str, versions: &[&str]) -> Option<Vec<f64>> {
        self.inner.estimates(context, versions)
    }

    fn can_estimate(&self) -> bool {
        self.inner.can_estimate()
    }
}

impl Logged<'_> {
    fn record(&self, question: &str, options: &[&str], p: &[f64]) {
        use emdysi_check::assist::*;
        let argmax = |v: &[f64]| {
            (0..v.len())
                .max_by(|&a, &b| v[a].total_cmp(&v[b]))
                .unwrap_or(0)
        };
        let right = argmax(&self.oracle.answer(question, options));
        let kind = if question.starts_with(LOCATE_QUESTION) {
            "locate"
        } else if question.starts_with(SCREEN_QUESTION) {
            "screen"
        } else if question == GRAMMAR_QUESTION {
            "grammar"
        } else if question == SPELLING_QUESTION {
            "spelling"
        } else {
            "other"
        };
        let all = p
            .iter()
            .map(|x| format!("{x:.4}"))
            .collect::<Vec<_>>()
            .join(",");
        self.log.push(format!(
            "{}\t{kind}\t{}\t{right}\t{}\t{:.4}\t{:.4}\t{all}",
            self.tag,
            options.len(),
            argmax(p),
            p[right],
            p[0]
        ));
    }
}

/// The perfect answer to `LOCATE_QUESTION`: the word where `sentence`
/// first differs from the corrected sentence most like it (for an
/// insertion, the word after the gap), as one-hot probabilities over the
/// word labels.
fn locate(sentence: &str, truth: &str, options: &[&str]) -> Vec<f64> {
    let b: Vec<char> = sentence.chars().collect();
    let common = |t: &str| {
        let a: Vec<char> = t.chars().collect();
        let p = b.iter().zip(&a).take_while(|(x, y)| x == y).count();
        let s = b[p..]
            .iter()
            .rev()
            .zip(a[p.min(a.len())..].iter().rev())
            .take_while(|(x, y)| x == y)
            .count();
        (p + s, p)
    };
    let best = truth.lines().max_by_key(|t| common(t)).unwrap_or("");
    let p = common(best).1;
    // Letters added at the end of a word (|amount| to |amounts|) are a
    // change of that word, not of the next one.
    let extends = p > 0
        && b[p - 1].is_alphanumeric()
        && best.chars().nth(p).is_some_and(|c| c.is_alphanumeric())
        && !b.get(p).is_some_and(|c| c.is_alphanumeric());
    // Whitespace-separated words and their spans.
    let mut spans = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        while i < b.len() && !b[i].is_whitespace() {
            i += 1;
        }
        spans.push((start, i));
    }
    let core = |&(f, t): &(usize, usize)| -> String {
        b[f..t]
            .iter()
            .collect::<String>()
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_string()
    };
    let k = spans
        .iter()
        .position(|&(_, t)| p < t || (extends && p == t))
        .unwrap_or(spans.len().saturating_sub(1));
    let word = spans.get(k).map(core).unwrap_or_default();
    let seen = spans[..k].iter().filter(|s| core(s) == word).count();
    let label = if seen == 0 {
        word
    } else {
        format!("{word} ({})", seen + 1)
    };
    let right = options.iter().position(|o| *o == label).unwrap_or(0);
    (0..options.len())
        .map(|i| if i == right { 1.0 } else { 0.0 })
        .collect()
}

/// Lines appended to a file as they come (the journal, `LOG`).
struct Appender(Mutex<std::fs::File>);

impl Appender {
    /// Opens `path` for appending, writing `header` first to a new file.
    fn open(path: &str, header: &str) -> Appender {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        if !header.is_empty() && f.metadata().unwrap().len() == 0 {
            writeln!(f, "{header}").unwrap();
        }
        Appender(Mutex::new(f))
    }

    fn push(&self, line: String) {
        use std::io::Write;
        writeln!(self.0.lock().unwrap(), "{line}").unwrap();
    }
}
