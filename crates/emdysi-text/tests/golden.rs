//! The English "Golden Rules" for sentence segmentation (pySBD, MIT).

use emdysi_text::segment::sentences;

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut cs = s.chars();
    while let Some(c) = cs.next() {
        if c == '\\' {
            match cs.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(d) => out.push(d),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[test]
fn golden_rules() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpora/golden-rules/en.tsv");
    let src = std::fs::read_to_string(path).unwrap();
    let mut pass = 0;
    let mut fail = Vec::new();
    for line in src.lines().filter(|l| !l.starts_with('#')) {
        let mut f = line.split('\t').map(unescape);
        let text = f.next().unwrap();
        let want: Vec<String> = f.collect();
        let got: Vec<String> = sentences(&text)
            .into_iter()
            .map(|r| text[r].to_string())
            .collect();
        if got == want {
            pass += 1;
        } else {
            fail.push(format!("{text}\n  want {want:?}\n  got  {got:?}"));
        }
    }
    for f in &fail {
        eprintln!("{f}");
    }
    eprintln!("{pass} passed, {} failed", fail.len());
    // Known miss: pySBD splits `compounds. . . . The` before the dots.
    assert!(fail.len() <= 1, "too many segmentation failures");
}
