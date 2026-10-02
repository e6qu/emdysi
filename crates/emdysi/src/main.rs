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
    en packs                       list built-in rule packs and rules

OPTIONS:
    --pack NAME|FILE       rule pack to use (repeatable; default: core, ai-tells, plain-style, substance)
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
    --samples N            with `rewrite`, model rewrites per sentence (default 3)
    --grammar DIR          grammar directory (default: the bundled ERG)
    --fail-on error|warning|suggestion  exit with status 1 if a diagnostic this
                           severe or worse is found (default: error)
";

struct Args {
    command: String,
    files: Vec<PathBuf>,
    packs: Vec<String>,
    disabled: Vec<String>,
    input: Option<Format>,
    output: OutputFormat,
    opts: Options,
    show: ParseDetails,
    model: Option<PathBuf>,
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
        disabled: Vec::new(),
        input: None,
        output: OutputFormat::Plain,
        opts: Options::default(),
        show: ParseDetails::default(),
        model: None,
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
        BUILTIN_PACKS.iter().map(|(n, _)| n.to_string()).collect()
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
            }
        }
        return Ok(true);
    }
    if !matches!(args.command.as_str(), "check" | "fix" | "parse" | "rewrite") {
        return Err(format!("unknown command {:?}\n\n{USAGE}", args.command));
    }
    let packs = load_packs(&args.packs)?;
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

#[cfg(feature = "llama")]
fn load_model(args: &Args) -> Result<Option<Box<dyn emdysi_rewrite::LanguageModel>>, String> {
    match &args.model {
        None => Ok(None),
        Some(p) => emdysi_rewrite::llama::LlamaLm::load(p, 2048)
            .map(|m| Some(Box::new(m) as Box<dyn emdysi_rewrite::LanguageModel>))
            .map_err(|e| format!("loading model: {e}")),
    }
}

#[cfg(not(feature = "llama"))]
fn load_model(args: &Args) -> Result<Option<Box<dyn emdysi_rewrite::LanguageModel>>, String> {
    match &args.model {
        None => Ok(None),
        Some(_) => Err(
            "--model needs a build with the `llama` feature: cargo install --features llama".into(),
        ),
    }
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
