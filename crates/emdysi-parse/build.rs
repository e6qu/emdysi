//! Hashes the source of the parsing pipeline (this crate, the HPSG engine,
//! REPP, the TDL reader and the ranking model) into `EMDYSI_PARSER_SOURCE`,
//! part of the key of cached parses: a parse cached by one version of the
//! code is never read by another.
//!
//! Also embeds the grammar (`grammar/erg` and `grammar/emdysi`), so that a
//! binary built from the repository runs without it (see
//! `emdysi_parse::grammar_dir`).

use std::path::{Path, PathBuf};

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            files(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dirs = [
        root.join("src"),
        root.join("data"),
        root.join("../emdysi-hpsg/src"),
        root.join("../emdysi-repp/src"),
        root.join("../emdysi-tdl/src"),
    ];
    let mut all = Vec::new();
    for d in &dirs {
        println!("cargo:rerun-if-changed={}", d.display());
        files(d, &mut all);
    }
    all.sort();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for p in &all {
        let rel = p.strip_prefix(root).unwrap_or(p);
        for b in rel
            .to_string_lossy()
            .bytes()
            .chain(std::fs::read(p).unwrap_or_default())
        {
            h = (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    println!("cargo:rustc-env=EMDYSI_PARSER_SOURCE={h:016x}");
    embed_grammar(root);
}

/// Writes `grammar_files.rs` to `OUT_DIR`: every file of the grammar, by
/// its path under `grammar/`, and a hash of them all.
fn embed_grammar(root: &Path) {
    let grammar = root.join("../../grammar");
    let mut all = Vec::new();
    for d in ["erg", "emdysi"] {
        let d = grammar.join(d);
        println!("cargo:rerun-if-changed={}", d.display());
        files(&d, &mut all);
    }
    all.retain(|p| {
        !p.file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with('.'))
    });
    all.sort();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut list = String::new();
    for p in &all {
        let rel = p
            .strip_prefix(&grammar)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        for b in rel.bytes().chain(std::fs::read(p).unwrap()) {
            h = (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
        }
        let abs = std::fs::canonicalize(p).unwrap();
        list.push_str(&format!("    ({rel:?}, include_bytes!({abs:?})),\n"));
    }
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("grammar_files.rs");
    std::fs::write(
        out,
        format!(
            "/// A hash of the embedded grammar's files.\n\
             pub(crate) const GRAMMAR_HASH: &str = \"{h:016x}\";\n\
             /// The embedded grammar: each file by its path under `grammar/`.\n\
             pub(crate) static GRAMMAR_FILES: &[(&str, &[u8])] = &[\n{list}];\n"
        ),
    )
    .unwrap();
}
