fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../grammar/erg");
    let repp = emdysi_repp::erg(&dir).unwrap();
    for line in std::io::stdin().lines() {
        let line = line.unwrap();
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
