//! Work (unifications tried) and time per parse, one line per sentence on
//! stdin: words, work, milliseconds, whether a strict reading was found,
//! whether the search was cut short.
use std::io::BufRead;
use std::time::Duration;

fn main() {
    let erg = emdysi_parse::Erg::load(&emdysi_parse::default_grammar_dir()).unwrap();
    let timeout = Duration::from_millis(
        std::env::var("TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10_000),
    );
    for line in std::io::stdin().lock().lines() {
        let text = line.unwrap();
        let Ok(p) = erg.parse_limited(&text, timeout, 3) else {
            continue;
        };
        let strict = p.readings.iter().any(|r| emdysi_parse::is_strict(&r.root));
        println!(
            "{}\t{}\t{}\t{}\t{}",
            text.split_whitespace().count(),
            p.work,
            p.elapsed.as_millis(),
            strict,
            p.exhausted
        );
    }
}
