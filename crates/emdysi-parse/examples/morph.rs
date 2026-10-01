use emdysi_parse::*;
fn main() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    for w in std::env::args().skip(1) {
        let an = erg
            .lexicon
            .morph
            .analyze(&w, &|s| erg.lexicon.is_stem(s), 3);
        println!("{w}: known={} {:?}", erg.known_word(&w), an);
    }
}
