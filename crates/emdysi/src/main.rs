//! `en`: check English prose in plain text or Markdown.

use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use emdysi_check::report::{OutputFormat, ParseDetails, render, render_parses};
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
    en packs                       list rule packs and rules (default packs, or
                                   those given with --pack)

OPTIONS:
    --pack NAME|FILE       rule pack to use (repeatable; default: core, ai-tells,
                           plain-style, substance, structure, terms; also
                           built in: microsoft, google, elastic, wordlists,
                           equality)
    --glossary FILE        project glossary: [[concept]] tables of preferred,
                           admitted and deprecated terms (repeatable)
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
    disabled: Vec<String>,
    input: Option<Format>,
    output: OutputFormat,
    opts: Options,
    show: ParseDetails,
    model: Option<PathBuf>,
    server: Option<String>,
    server_model: Option<String>,
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
        disabled: Vec::new(),
        input: None,
        output: OutputFormat::Plain,
        opts: Options::default(),
        show: ParseDetails::default(),
        model: None,
        server: None,
        server_model: None,
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
    if !matches!(args.command.as_str(), "check" | "fix" | "parse" | "rewrite") {
        return Err(format!("unknown command {:?}\n\n{USAGE}", args.command));
    }
    let mut packs = load_packs(&args.packs)?;
    for g in &args.glossaries {
        let src = std::fs::read_to_string(g).map_err(|e| format!("{}: {e}", g.display()))?;
        packs.push(Pack::parse_glossary(&src).map_err(|e| format!("{}: {e}", g.display()))?);
    }
    let docs = inputs(&args.files, args.input)?;
    let erg = Erg::load(&args.grammar).map_err(|e| format!("loading grammar: {e}"))?;
    let mut checker = Checker::new(packs);
    checker.disabled = args.disabled.clone();
    let mut model = load_model(&args)?;
    let mut ok = true;
    for (name, src, format) in docs {
        let a = analyze(&erg, &src, format, &args.opts);
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
                        Some(m) => Some(&mut **m as &mut dyn emdysi_rewrite::LanguageModel),
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
                eprintln!("{name}: {n} sentences rewritten");
            }
            "fix" => {
                let diags = checker.check(&erg, &a);
                let (fixed, n) = apply_fixes(&src, &diags);
                print!("{fixed}");
                eprintln!("{name}: {n} fixes applied");
            }
            _ => {
                let diags = checker.check(&erg, &a);
                if diags.iter().any(|d| d.severity <= args.fail_on) {
                    ok = false;
                }
                print!("{}", render(&a, &diags, &name, args.output));
            }
        }
    }
    Ok(ok)
}

fn load_model(args: &Args) -> Result<Option<Box<dyn emdysi_rewrite::LanguageModel>>, String> {
    if args.model.is_some() && args.server.is_some() {
        return Err("use either --model or --server, not both".into());
    }
    if let Some(url) = &args.server {
        return Ok(Some(Box::new(emdysi_rewrite::http::HttpLm::new(
            url,
            args.server_model.clone(),
        ))));
    }
    match &args.model {
        None => Ok(None),
        Some(p) => load_gguf(p),
    }
}

#[cfg(feature = "llama")]
fn load_gguf(
    p: &std::path::Path,
) -> Result<Option<Box<dyn emdysi_rewrite::LanguageModel>>, String> {
    emdysi_rewrite::llama::LlamaLm::load(p, 2048)
        .map(|m| Some(Box::new(m) as Box<dyn emdysi_rewrite::LanguageModel>))
        .map_err(|e| format!("loading model: {e}"))
}

#[cfg(not(feature = "llama"))]
fn load_gguf(
    _: &std::path::Path,
) -> Result<Option<Box<dyn emdysi_rewrite::LanguageModel>>, String> {
    Err("--model needs a build with the `llama` feature: cargo install --features llama (or serve the model, e.g. with MLX, and use --server)".into())
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
