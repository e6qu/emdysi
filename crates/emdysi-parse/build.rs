//! Hashes the source of the parsing pipeline (this crate, the HPSG engine,
//! REPP, the TDL reader and the ranking model) into `EMDYSI_PARSER_SOURCE`,
//! part of the key of cached parses: a parse cached by one version of the
//! code is never read by another.

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
}
