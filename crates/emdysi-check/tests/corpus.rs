//! Regression test over the AI-style prose samples in `corpora/ai-prose`:
//! the diagnostics of every built-in pack are compared with
//! `expected/<name>.txt`. Set `UPDATE_EXPECTED=1` to rewrite those files.

use std::fmt::Write;
use std::path::Path;
use std::time::Duration;

use emdysi_check::*;
use emdysi_parse::{Erg, default_grammar_dir};

fn packs() -> Vec<Pack> {
    ["core", "ai-tells", "plain-style", "substance"]
        .iter()
        .map(|p| {
            let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../packs/{p}.toml"));
            Pack::parse(&std::fs::read_to_string(path).unwrap()).unwrap()
        })
        .collect()
}

#[test]
fn ai_prose_corpus() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpora/ai-prose");
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let checker = Checker::new(packs());
    // Generous limits so that slow machines give the same analyses.
    let opts = Options {
        threads: 2,
        timeout: Duration::from_secs(60),
        ..Options::default()
    };
    let update = std::env::var("UPDATE_EXPECTED").is_ok();
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "md") && !p.ends_with("README.md"))
        .collect();
    files.sort();
    let mut failures = Vec::new();
    for path in files {
        let src = std::fs::read_to_string(&path).unwrap();
        let a = analyze(&erg, &src, Format::Markdown, &opts);
        let mut got = String::new();
        for d in checker.check(&erg, &a) {
            let (line, col) = a.line_col(d.range.start);
            write!(
                got,
                "{line}:{col} {} {} {:?}",
                d.severity.as_str(),
                d.rule,
                &src[d.range.clone()]
            )
            .unwrap();
            if let Some(r) = &d.replacement {
                write!(got, " -> {r:?}").unwrap();
            }
            got.push('\n');
        }
        let name = path.file_stem().unwrap().to_string_lossy().to_string();
        let expected_path = dir.join("expected").join(format!("{name}.txt"));
        if update {
            std::fs::write(&expected_path, &got).unwrap();
            continue;
        }
        let expected = std::fs::read_to_string(&expected_path).unwrap_or_default();
        if got != expected {
            failures.push(format!("{name}:\n--- expected\n{expected}--- got\n{got}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
