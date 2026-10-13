fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../grammar/erg");
    let mut repp = emdysi_repp::erg(&dir).unwrap();
    // EXT=path adds a grammar's own rules, as emdysi adds
    // grammar/emdysi/tokenizer.rpp.
    if let Some(ext) = std::env::var_os("EXT") {
        repp.extend(std::path::Path::new(&ext)).unwrap();
    }
    let trace = std::env::var_os("TRACE").is_some();
    for line in std::io::stdin().lines() {
        let line = line.unwrap();
        if trace {
            for (rule, after) in repp.trace(&line).1 {
                println!("  {rule}\n    -> {after}");
            }
        }
        let toks = repp.tokenize(&line);
        println!(
            "{}",
            toks.iter()
                .map(|t| format!("{}<{}:{}>", t.form, t.from, t.to))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
}
