use emdysi_repp::*;

fn repp() -> Repp {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../grammar/erg");
    erg(&dir).unwrap()
}

fn show(r: &Repp, s: &str) -> String {
    r.tokenize(s)
        .iter()
        .map(|t| format!("{}<{}:{}>", t.form, t.from, t.to))
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn erg_tokenization() {
    let r = repp();
    assert_eq!(show(&r, "It rained."), "It<0:2> rained<3:9> .<9:10>");
    assert_eq!(
        show(&r, "Kim's dog didn't bark, \"really\" (or so)."),
        "Kim<0:3> ’s<3:5> dog<6:9> did<10:13> n’t<13:16> bark<17:21> ,<21:22> “<23:24> \
         really<24:30> ”<30:31> (<32:33> or<33:35> so<36:38> )<38:39> .<39:40>"
    );
    assert_eq!(
        show(&r, "The 3-4 dogs cost US$5... ok?"),
        "The<0:3> 3<4:5> –<5:6> 4<6:7> dogs<8:12> cost<13:17> US$<18:21> 5<21:22> …<22:25> ok<26:28> ?<28:29>"
    );
}

#[test]
fn offsets_index_original_characters() {
    let r = repp();
    let input = "Café owners’ dogs — barked.";
    let chars: Vec<char> = input.chars().collect();
    for t in r.tokenize(input) {
        let orig: String = chars[t.from..t.to].iter().collect();
        assert!(!orig.trim().is_empty(), "{t:?}");
    }
}
