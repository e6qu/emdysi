//! `en`: check English prose in plain text or Markdown.

use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use emdysi_check::report::{OutputFormat, ParseDetails, count, render, render_parses};
use emdysi_check::{Checker, Format, Options, Pack, Severity, analyze, apply_fixes};
use emdysi_parse::{Erg, default_grammar_dir};

const BUILTIN_PACKS: &[(&str, &str)] = &[
    ("core", include_str!("../../../packs/core.toml")),
    ("ai-tells", include_str!("../../../packs/ai-tells.toml")),
    (
        "plain-style",
        include_str!("../../../packs/plain-style.toml"),
    ),
    ("substance", include_str!("../../../packs/substance.toml")),
    ("structure", include_str!("../../../packs/structure.toml")),
    ("terms", include_str!("../../../packs/terms.toml")),
    // Opt-in packs generated from other linters' rule data
    // (scripts/import-rules.py).
    ("microsoft", include_str!("../../../packs/microsoft.toml")),
    ("google", include_str!("../../../packs/google.toml")),
    ("elastic", include_str!("../../../packs/elastic.toml")),
    ("wordlists", include_str!("../../../packs/wordlists.toml")),
    ("equality", include_str!("../../../packs/equality.toml")),
    // Sentences the grammar could not analyse fully (not errors).
    ("coverage", include_str!("../../../packs/coverage.toml")),
    // Questions for a local decision model; nothing without one.
    ("decisions", include_str!("../../../packs/decisions.toml")),
];

/// Packs used when no `--pack` is given.
const DEFAULT_PACKS: &[&str] = &[
    "core",
    "ai-tells",
    "plain-style",
    "substance",
    "structure",
    "terms",
];

const USAGE: &str = "\
en: grammar, spelling, style and AI-writing checks for English prose

USAGE:
    en check [OPTIONS] [FILE...]   report problems (stdin if no file)
    en fix   [OPTIONS] [FILE...]   print the text with automatic fixes applied
    en parse [OPTIONS] [FILE...]   show sentence analyses
    en rewrite [OPTIONS] [FILE...] print the text with guarded rewrites: fixes,
                                   spelling corrections and (with --model) a
                                   local model's rewrites, each kept only if
                                   the grammar accepts it and its meaning holds
    en decide [MODEL] --question Q --option X --option Y ... [--context TEXT]
    en decide [MODEL] --statement S [--context TEXT]
    en decide [MODEL] --question Q --scale LOW..HIGH [--context TEXT]
    en decide [MODEL] --eval FILE.tsv
                                   ask a local model a question with fixed
                                   answers and print their probabilities;
                                   --eval measures accuracy and calibration on
                                   questions with known answers (TSV: question,
                                   context, options separated by '|', index of
                                   the right one) and fits a temperature
    en glossary [--to toml|tbx] FILE...
                                   convert glossaries (TOML, TBX, Vale
                                   vocabularies) and print them
    en packs                       list rule packs and rules (default packs, or
                                   those given with --pack)

OPTIONS:
    --pack NAME|FILE       rule pack to use (repeatable; default: core, ai-tells,
                           plain-style, substance, structure, terms; also
                           built in: microsoft, google, elastic, wordlists,
                           equality, and decisions, which needs a model)
    --glossary FILE        project glossary (repeatable): TOML ([[concept]]
                           tables), TBX (.tbx, .xml) or a Vale vocabulary
                           (a directory with accept.txt and reject.txt)
    --disable RULE         skip a rule id, or a prefix ending in '*' (repeatable)
    --input plain|markdown input format (default: from the file extension; stdin is plain)
    --format plain|markdown  output format (default: plain)
    --no-parse             tokenize only; skip grammar-based checks
    --max-tokens N         do not parse sentences longer than N tokens (default 100)
    --timeout SECS         time limit per sentence (default 10)
    --derivations          with `parse`, also print derivation trees
    --mrs                  with `parse`, also print the semantics (MRS)
    --model FILE.gguf      with `rewrite`, a local language model (needs a
                           build with the `llama` feature)
    --server URL           with `rewrite`, a model served over the
                           OpenAI-compatible API, e.g. MLX's `mlx_lm.server`
                           or LM Studio (http://127.0.0.1:8080)
    --server-model NAME    the model name to request from --server
    --lm SPEC              any model: FILE.gguf, http://HOST:PORT[#MODEL] or
                           script:FILE (a scripted stand-in)
    --decide-readings      with a model and `check`, `fix` or `parse`: when the
                           two best readings of a sentence are close, ask the
                           model which grouping of words is meant
    --no-context-readings  with `check`, `fix` or `parse`: do not let the rest of
                           the document settle close calls between readings
                           (by default, of readings that score about the same,
                           the one grouping words as the document does elsewhere
                           is preferred)
    --no-debias            with `decide`, ask once instead of also with the
                           options reversed
    --temperature T        with `decide`, calibration temperature (default 1)
    --samples N            with `rewrite`, model rewrites per sentence (default 3)
    --grammar DIR          grammar directory (default: the bundled ERG)
    --fail-on error|warning|suggestion  exit with status 1 if a diagnostic this
                           severe or worse is found (default: error)
";

struct Args {
    command: String,
    files: Vec<PathBuf>,
    packs: Vec<String>,
    glossaries: Vec<PathBuf>,
    to: Option<String>,
    disabled: Vec<String>,
    input: Option<Format>,
    output: OutputFormat,
    opts: Options,
    show: ParseDetails,
    model: Option<PathBuf>,
    server: Option<String>,
    server_model: Option<String>,
    lm: Option<String>,
    question: Option<String>,
    context: String,
    options: Vec<String>,
    statement: Option<String>,
    scale: Option<(i32, i32)>,
    eval: Option<PathBuf>,
    debias: bool,
    temperature: f64,
    decide_readings: bool,
    context_readings: bool,
    samples: usize,
    grammar: PathBuf,
    fail_on: Severity,
}

fn parse_args() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let command = it.next().ok_or_else(|| USAGE.to_string())?;
    if command == "-h" || command == "--help" || command == "help" {
        return Err(USAGE.to_string());
    }
    let mut a = Args {
        command,
        files: Vec::new(),
        packs: Vec::new(),
        glossaries: Vec::new(),
        to: None,
        disabled: Vec::new(),
        input: None,
        output: OutputFormat::Plain,
        opts: Options::default(),
        show: ParseDetails::default(),
        model: None,
        server: None,
        server_model: None,
        lm: None,
        question: None,
        context: String::new(),
        options: Vec::new(),
        statement: None,
        scale: None,
        eval: None,
        debias: true,
        temperature: 1.0,
        decide_readings: false,
        context_readings: true,
        samples: 3,
        grammar: default_grammar_dir(),
        fail_on: Severity::Error,
    };
    let need = |it: &mut std::iter::Skip<std::env::Args>, flag: &str| {
        it.next().ok_or_else(|| format!("{flag} needs a value"))
    };
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--pack" => a.packs.push(need(&mut it, &arg)?),
            "--glossary" => a.glossaries.push(PathBuf::from(need(&mut it, &arg)?)),
            "--disable" => a.disabled.push(need(&mut it, &arg)?),
            "--input" => {
                a.input = Some(match need(&mut it, &arg)?.as_str() {
                    "plain" | "text" => Format::Plain,
                    "markdown" | "md" => Format::Markdown,
                    v => return Err(format!("unknown input format {v:?}")),
                })
            }
            "--format" => {
                a.output = match need(&mut it, &arg)?.as_str() {
                    "plain" | "text" => OutputFormat::Plain,
                    "markdown" | "md" => OutputFormat::Markdown,
                    v => return Err(format!("unknown output format {v:?}")),
                }
            }
            "--no-parse" => a.opts.parse = false,
            "--max-tokens" => {
                a.opts.max_tokens = need(&mut it, &arg)?
                    .parse()
                    .map_err(|_| "--max-tokens needs a number")?
            }
            "--timeout" => {
                a.opts.timeout = Duration::from_secs_f64(
                    need(&mut it, &arg)?
                        .parse()
                        .map_err(|_| "--timeout needs a number")?,
                )
            }
            "--derivations" => a.show.derivations = true,
            "--mrs" => a.show.mrs = true,
            "--model" => a.model = Some(PathBuf::from(need(&mut it, &arg)?)),
            "--server" => a.server = Some(need(&mut it, &arg)?),
            "--server-model" => a.server_model = Some(need(&mut it, &arg)?),
            "--lm" => a.lm = Some(need(&mut it, &arg)?),
            "--question" => a.question = Some(need(&mut it, &arg)?),
            "--context" => a.context = need(&mut it, &arg)?,
            "--option" => a.options.push(need(&mut it, &arg)?),
            "--statement" => a.statement = Some(need(&mut it, &arg)?),
            "--scale" => {
                let v = need(&mut it, &arg)?;
                let (lo, hi) = v
                    .split_once("..")
                    .and_then(|(l, h)| Some((l.parse().ok()?, h.parse().ok()?)))
                    .ok_or("--scale needs LOW..HIGH, e.g. 1..5")?;
                a.scale = Some((lo, hi));
            }
            "--eval" => a.eval = Some(PathBuf::from(need(&mut it, &arg)?)),
            "--no-debias" => a.debias = false,
            "--decide-readings" => a.decide_readings = true,
            "--no-context-readings" => a.context_readings = false,
            "--temperature" => {
                a.temperature = need(&mut it, &arg)?
                    .parse()
                    .map_err(|_| "--temperature needs a number")?
            }
            "--to" => a.to = Some(need(&mut it, &arg)?),
            "--samples" => {
                a.samples = need(&mut it, &arg)?
                    .parse()
                    .map_err(|_| "--samples needs a number")?
            }
            "--grammar" => a.grammar = PathBuf::from(need(&mut it, &arg)?),
            "--fail-on" => {
                a.fail_on = Severity::parse(&need(&mut it, &arg)?)
                    .ok_or("--fail-on needs error, warning or suggestion")?
            }
            "-h" | "--help" => return Err(USAGE.to_string()),
            f if f.starts_with("--") => return Err(format!("unknown option {f}\n\n{USAGE}")),
            f => a.files.push(PathBuf::from(f)),
        }
    }
    Ok(a)
}

/// A glossary from a TOML file, a TBX file (`.tbx`, `.xml`), or a Vale
/// vocabulary (a directory with `accept.txt` and `reject.txt`, or one of
/// those files).
fn load_glossary(path: &std::path::Path) -> Result<Pack, String> {
    let err = |e: String| format!("{}: {e}", path.display());
    let read = |p: &std::path::Path| std::fs::read_to_string(p).unwrap_or_default();
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let vale = |accept: &str, reject: &str| {
        let (concepts, skipped) = emdysi_check::glossary::from_vale_vocab(accept, reject);
        if !skipped.is_empty() {
            eprintln!(
                "{}: {} regular-expression entries not converted: {}",
                path.display(),
                skipped.len(),
                skipped.join(", ")
            );
        }
        Pack::from_concepts(concepts)
    };
    if path.is_dir() {
        return Ok(vale(
            &read(&path.join("accept.txt")),
            &read(&path.join("reject.txt")),
        ));
    }
    match name {
        "accept.txt" => return Ok(vale(&read(path), "")),
        "reject.txt" => return Ok(vale("", &read(path))),
        _ => {}
    }
    let src = std::fs::read_to_string(path).map_err(|e| err(e.to_string()))?;
    match path.extension().and_then(|e| e.to_str()) {
        Some("tbx" | "xml") => emdysi_check::glossary::from_tbx(&src)
            .map(Pack::from_concepts)
            .map_err(err),
        _ => Pack::parse_glossary(&src).map_err(|e| err(e.to_string())),
    }
}

fn load_packs(names: &[String]) -> Result<Vec<Pack>, String> {
    let names: Vec<String> = if names.is_empty() {
        DEFAULT_PACKS.iter().map(|n| n.to_string()).collect()
    } else {
        names.to_vec()
    };
    names
        .iter()
        .map(|n| {
            let src = match BUILTIN_PACKS.iter().find(|(b, _)| b == n) {
                Some((_, src)) => src.to_string(),
                None => std::fs::read_to_string(n).map_err(|e| format!("{n}: {e}"))?,
            };
            Pack::parse(&src).map_err(|e| format!("{n}: {e}"))
        })
        .collect()
}

fn inputs(
    files: &[PathBuf],
    input: Option<Format>,
) -> Result<Vec<(String, String, Format)>, String> {
    if files.is_empty() {
        let mut s = String::new();
        std::io::stdin()
            .read_to_string(&mut s)
            .map_err(|e| e.to_string())?;
        return Ok(vec![("<stdin>".into(), s, input.unwrap_or(Format::Plain))]);
    }
    files
        .iter()
        .map(|f| {
            let s = std::fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display()))?;
            Ok((
                f.display().to_string(),
                s,
                input.unwrap_or_else(|| Format::from_path(f)),
            ))
        })
        .collect()
}

fn run() -> Result<bool, String> {
    let args = parse_args()?;
    if args.command == "packs" {
        for p in load_packs(&args.packs)? {
            println!("{}: {}", p.name, p.description);
            for r in &p.rules {
                println!("  {} ({})", r.id, r.severity.as_str());
                if let Some(src) = &r.source {
                    println!("      source: {src}");
                }
            }
        }
        return Ok(true);
    }
    if args.command == "glossary" {
        let mut concepts = Vec::new();
        for f in &args.files {
            concepts.extend(load_glossary(f)?.concepts);
        }
        match args.to.as_deref() {
            None | Some("toml") => print!("{}", emdysi_check::glossary::to_toml(&concepts)),
            Some("tbx") => print!("{}", emdysi_check::glossary::to_tbx(&concepts)),
            Some(o) => return Err(format!("--to must be toml or tbx, not {o:?}")),
        }
        return Ok(true);
    }
    if args.command == "decide" {
        return decide(&args);
    }
    if !matches!(args.command.as_str(), "check" | "fix" | "parse" | "rewrite") {
        return Err(format!("unknown command {:?}\n\n{USAGE}", args.command));
    }
    let mut packs = load_packs(&args.packs)?;
    for g in &args.glossaries {
        packs.push(load_glossary(g)?);
    }
    let docs = inputs(&args.files, args.input)?;
    let erg = Erg::load(&args.grammar).map_err(|e| format!("loading grammar: {e}"))?;
    let mut checker = Checker::new(packs);
    checker.disabled = args.disabled.clone();
    let mut model = load_model(&args)?;
    let mut ok = true;
    if args.decide_readings && model.is_none() {
        return Err("--decide-readings needs a model (--model, --server or --lm)".into());
    }
    for (name, src, format) in docs {
        let mut a = analyze(&erg, &src, format, &args.opts);
        if args.context_readings {
            emdysi_check::decisions::prefer_document_phrases(&mut a, READING_MARGIN);
        }
        if let (true, Some(m)) = (args.decide_readings, model.as_mut()) {
            let mut ask = ModelAsk::new(&mut **m, &args);
            let n = emdysi_check::decisions::disambiguate(&mut a, &mut ask, READING_MARGIN, 0.6);
            eprintln!(
                "{name}: the model changed the best reading of {}",
                count(n, "sentence", "sentences")
            );
        }
        let a = a;
        match args.command.as_str() {
            "parse" => print!("{}", render_parses(&a, args.output, &args.show)),
            "rewrite" => {
                let diags = checker.check(&erg, &a);
                let mut rw = emdysi_rewrite::Rewriter {
                    erg: &erg,
                    checker: &checker,
                    options: args.opts.clone(),
                    rewrite: emdysi_rewrite::RewriteOptions {
                        samples: args.samples,
                        ..Default::default()
                    },
                    model: match model.as_mut() {
                        Some(m) => Some(&mut **m as &mut dyn emdysi_lm::LanguageModel),
                        None => None,
                    },
                };
                let proposals = rw.propose(&a, &diags);
                let (out, n) = emdysi_rewrite::apply(&a, &proposals);
                print!("{out}");
                for p in &proposals {
                    match &p.chosen {
                        Some(c) => eprintln!(
                            "{name}: rewrote ({:?}): {} -> {}",
                            c.source, p.original, c.text
                        ),
                        None => eprintln!(
                            "{name}: kept: {} ({} candidate(s) rejected)",
                            p.original,
                            p.candidates.len()
                        ),
                    }
                }
                eprintln!("{name}: {} rewritten", count(n, "sentence", "sentences"));
            }
            "fix" => {
                let diags = check(&checker, &erg, &a, model.as_mut(), &args);
                let (fixed, n) = apply_fixes(&src, &diags);
                print!("{fixed}");
                eprintln!("{name}: {} applied", count(n, "fix", "fixes"));
            }
            _ => {
                let diags = check(&checker, &erg, &a, model.as_mut(), &args);
                if diags.iter().any(|d| d.severity <= args.fail_on) {
                    ok = false;
                }
                print!("{}", render(&a, &diags, &name, args.output));
            }
        }
    }
    Ok(ok)
}

fn load_model(args: &Args) -> Result<Option<Box<dyn emdysi_lm::LanguageModel>>, String> {
    let given = [
        args.model.is_some(),
        args.server.is_some(),
        args.lm.is_some(),
    ];
    if given.iter().filter(|&&g| g).count() > 1 {
        return Err("give one of --model, --server and --lm".into());
    }
    if let Some(url) = &args.server {
        let spec = match &args.server_model {
            Some(m) => format!("{url}#{m}"),
            None => url.clone(),
        };
        return emdysi_lm::open(&spec).map(Some);
    }
    if let Some(p) = &args.model {
        return emdysi_lm::open(&format!("gguf:{}", p.display())).map(Some);
    }
    args.lm.as_deref().map(emdysi_lm::open).transpose()
}

/// Readings whose ranker scores differ by at most this much are close
/// enough for the rest of the document or the model to decide between.
const READING_MARGIN: f64 = 2.0;

/// A decision model as the checker's [`emdysi_check::decisions::Ask`].
struct ModelAsk<'a> {
    d: emdysi_lm::decide::Decider<'a>,
}

impl<'a> ModelAsk<'a> {
    fn new(lm: &'a mut dyn emdysi_lm::LanguageModel, args: &Args) -> ModelAsk<'a> {
        let mut d = emdysi_lm::decide::Decider::new(lm);
        d.debias = args.debias;
        d.temperature = args.temperature;
        ModelAsk { d }
    }
}

impl emdysi_check::decisions::Ask for ModelAsk<'_> {
    fn ask(&mut self, context: &str, question: &str, options: &[&str]) -> Option<Vec<f64>> {
        self.d.choose(context, question, options).ok()
    }
}

/// Check, with the model for `decide` rules when there is one.
fn check(
    checker: &Checker,
    erg: &Erg,
    a: &emdysi_check::Analysis,
    model: Option<&mut Box<dyn emdysi_lm::LanguageModel>>,
    args: &Args,
) -> Vec<emdysi_check::Diagnostic> {
    match model {
        Some(m) => {
            let mut ask = ModelAsk::new(&mut **m, args);
            checker.check_with(erg, a, Some(&mut ask))
        }
        None => checker.check(erg, a),
    }
}

/// `en decide`: one question, or an evaluation file.
fn decide(args: &Args) -> Result<bool, String> {
    use emdysi_lm::decide::{Decider, Sample, fit_temperature, report};
    let mut lm = load_model(args)?
        .ok_or("decide needs a model: --model FILE.gguf, --server URL or --lm SPEC")?;
    let mut d = Decider::new(&mut *lm);
    d.debias = args.debias;
    d.temperature = args.temperature;
    if let Some(path) = &args.eval {
        let src = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut samples = Vec::new();
        for (i, line) in src.lines().enumerate() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            let [question, context, options, right] = f[..] else {
                return Err(format!(
                    "{}:{}: expected 4 tab-separated fields",
                    path.display(),
                    i + 1
                ));
            };
            let options: Vec<&str> = options.split('|').collect();
            let right: usize = right
                .trim()
                .parse()
                .map_err(|_| format!("{}:{}: bad answer index", path.display(), i + 1))?;
            let raw = d
                .raw(context, question, &options)
                .map_err(|e| e.to_string())?;
            samples.push(Sample { raw, right });
        }
        let t = fit_temperature(&samples);
        for (name, temp) in [("given", args.temperature), ("fitted", t)] {
            let r = report(&samples, temp);
            println!(
                "{name} temperature {temp:.3}: {} questions, accuracy {:.3}, log loss {:.3}, Brier {:.3}, calibration error {:.3}",
                r.n, r.accuracy, r.nll, r.brier, r.ece
            );
        }
        return Ok(true);
    }
    let rows: Vec<(String, f64)> = if let Some(s) = &args.statement {
        let p = d.holds(&args.context, s).map_err(|e| e.to_string())?;
        vec![("true".into(), p), ("false".into(), 1.0 - p)]
    } else {
        let q = args
            .question
            .as_deref()
            .ok_or("decide needs --question or --statement")?;
        if let Some((lo, hi)) = args.scale {
            d.score(&args.context, q, lo, hi)
                .map_err(|e| e.to_string())?
                .into_iter()
                .map(|(k, p)| (k.to_string(), p))
                .collect()
        } else {
            let opts: Vec<&str> = args.options.iter().map(String::as_str).collect();
            let p = d
                .choose(&args.context, q, &opts)
                .map_err(|e| e.to_string())?;
            args.options.iter().cloned().zip(p).collect()
        }
    };
    for (o, p) in rows {
        println!("{p:.4}\t{o}");
    }
    Ok(true)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(2)
        }
    }
}
