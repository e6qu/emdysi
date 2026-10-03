//! The decision engine against real backends, when they are available:
//!
//! - `EMDYSI_TEST_GGUF=FILE.gguf` (with `--features llama`): in-process
//!   llama.cpp;
//! - `EMDYSI_TEST_SERVERS=URL,URL`: OpenAI-compatible servers, such as
//!   `llama-server` and `mlx_lm.server`;
//! - `EMDYSI_TEST_SAME_WEIGHTS=1`: all of them run the same weights (e.g.
//!   the tiny model of `scripts/tiny-model/build.py` as GGUF and MLX), so
//!   their answers must agree.
//!
//! Without these variables the tests do nothing.

use emdysi_lm::decide::{Decider, Raw};
use emdysi_lm::{LanguageModel, open};

fn backends() -> Vec<(String, Box<dyn LanguageModel>)> {
    let mut out = Vec::new();
    if let Ok(path) = std::env::var("EMDYSI_TEST_GGUF") {
        match open(&format!("gguf:{path}")) {
            Ok(m) => out.push((path, m)),
            Err(e) if cfg!(feature = "llama") => panic!("{e}"),
            Err(_) => eprintln!("EMDYSI_TEST_GGUF needs --features llama; skipped"),
        }
    }
    if let Ok(urls) = std::env::var("EMDYSI_TEST_SERVERS") {
        for u in urls.split(',').filter(|u| !u.is_empty()) {
            out.push((u.to_string(), open(u).unwrap()));
        }
    }
    out
}

const QUESTIONS: &[(&str, &str, &[&str])] = &[
    (
        "",
        "Which spelling is American English?",
        &["colour", "color"],
    ),
    (
        "",
        "Which sentence is grammatical?",
        &["The cats sleeps.", "The cats sleep."],
    ),
    (
        "Our platform is a game-changer that unlocks synergies.",
        "How specific is this sentence?",
        &["1", "2", "3", "4", "5"],
    ),
];

fn answers(lm: &mut dyn LanguageModel) -> Vec<Raw> {
    let mut d = Decider::new(lm);
    QUESTIONS
        .iter()
        .map(|(c, q, o)| {
            let raw = d.raw(c, q, o).unwrap();
            let p = emdysi_lm::decide::probabilities(&raw, 1.0);
            assert_eq!(p.len(), o.len());
            assert!((p.iter().sum::<f64>() - 1.0).abs() < 1e-9, "{p:?}");
            assert!(p.iter().all(|&x| (0.0..=1.0).contains(&x)));
            raw
        })
        .collect()
}

#[test]
fn decisions_on_every_backend() {
    let mut all = Vec::new();
    for (name, mut lm) in backends() {
        let raw = answers(&mut *lm);
        // Generation works too.
        let g = lm.generate("Reply briefly.", "Say hello.", 8, 0.0, 1);
        assert!(g.is_some(), "{name}: no generation");
        eprintln!(
            "{name}: {:?}",
            raw.iter().map(|r| &r.orders[0]).collect::<Vec<_>>()
        );
        all.push((name, raw));
    }
    if std::env::var("EMDYSI_TEST_SAME_WEIGHTS").is_ok() {
        // Two-option questions only: in-process llama.cpp scores every
        // label exactly, a server only its top tokens. In-process, a label
        // also counts its spelling with a leading space (" A"), which a
        // server returns only when it is among its top tokens, so the
        // comparison with a server is loose; servers must agree closely
        // (a wrongly converted GGUF would not).
        let gguf = std::env::var("EMDYSI_TEST_GGUF").ok();
        for i in 0..all.len() {
            for j in i + 1..all.len() {
                let (na, ra) = &all[i];
                let (nb, rb) = &all[j];
                let loose =
                    gguf.as_deref() == Some(na.as_str()) || gguf.as_deref() == Some(nb.as_str());
                let tol = if loose { 0.1 } else { 1e-3 };
                for (a, b) in ra.iter().zip(rb).take(2) {
                    for (x, y) in a.orders.iter().flatten().zip(b.orders.iter().flatten()) {
                        assert!((x - y).abs() < tol, "{na} and {nb} disagree: {x} vs {y}");
                    }
                }
            }
        }
    }
}
