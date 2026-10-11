//! emdysi's grammar extensions (`grammar/emdysi`) admit what they are for
//! as formal English, and nothing next to it that edited English does not
//! write.

use emdysi_parse::{Erg, default_grammar_dir, is_strict};

fn strict(erg: &Erg, text: &str) -> bool {
    erg.parse(text)
        .unwrap()
        .readings
        .iter()
        .any(|r| is_strict(&r.root))
}

#[test]
fn paired_comma_coordination() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    for text in [
        "You, or a controller, must delete the object.",
        "Kim, and Sandy, left.",
        "The kubelet, or the API server, rejects the request.",
        "I saw Kim, and Sandy, yesterday.",
        "Kim and Sandy left.",
        "Kim, Sandy, and Lee left.",
    ] {
        assert!(strict(&erg, text), "{text}");
    }
    for text in [
        "Abrams, and Browne arrived.",
        "The dog, barked.",
        "I saw Kim, and Sandy.",
    ] {
        assert!(!strict(&erg, text), "{text}");
    }
}

#[test]
fn bracketed_adjunct_after_conjunction() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    for text in [
        "Kim stayed, but, in the end, Sandy left.",
        "Kim stayed, and, in the end, left.",
        "Bennet did not reply, but, unable to wait, began scolding.",
        "Kim stayed, but Sandy left.",
    ] {
        assert!(strict(&erg, text), "{text}");
    }
    for text in ["Kim and, Sandy left.", "I saw Kim and, Sandy."] {
        assert!(!strict(&erg, text), "{text}");
    }
}
