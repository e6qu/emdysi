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

#[test]
fn exclamations_without_a_verb() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    for text in [
        "What a contrast!",
        "What a contrast between him and his friend!",
        "Nonsense!",
    ] {
        assert!(strict(&erg, text), "{text}");
    }
    for text in ["The cat.", "What a contrast.", "Because it failed."] {
        assert!(!strict(&erg, text), "{text}");
    }
}

#[test]
fn comparative_with_a_shared_object() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    for text in [
        "It is similar to, but less extreme than, the climate.",
        "It is similar to and more extreme than the climate.",
        "Who is Kim less tall than?",
    ] {
        assert!(strict(&erg, text), "{text}");
    }
    for text in ["It is less extreme than.", "It is more extreme than."] {
        assert!(!strict(&erg, text), "{text}");
    }
}

#[test]
fn collective_nouns_with_plural_agreement() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    for text in [
        "The team are almost ready.",
        "The staff are busy.",
        "The family were divided.",
        "The team is almost ready.",
    ] {
        assert!(strict(&erg, text), "{text}");
    }
    for text in ["The cat are hungry.", "Kim are ready.", "I like team."] {
        assert!(!strict(&erg, text), "{text}");
    }
}

#[test]
fn ranges_of_amounts() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    let forms: Vec<String> = erg
        .tokens("£3,000-£5,000")
        .into_iter()
        .map(|t| t.form)
        .collect();
    assert_eq!(forms, ["£", "3,000", "–", "£", "5,000"]);
    for text in [
        "They earn 3% on balances of £3,000-£5,000.",
        "It costs $10-$20.",
    ] {
        assert!(strict(&erg, text), "{text}");
    }
}

#[test]
fn from_and_to_with_adjectives() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    for text in [
        "The leaves range from yellowish to bluish.",
        "It went from bad to worse.",
    ] {
        assert!(strict(&erg, text), "{text}");
    }
    for text in ["It went from happy.", "It belongs to happy."] {
        assert!(!strict(&erg, text), "{text}");
    }
}

#[test]
fn heavy_object_after_an_adjunct() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    for text in [
        "People express through their disapproval all their hatred of modern parenting.",
        "We saw yesterday all the cats that we had fed.",
        "He could, in her presence, be rude.",
        "The subject, about a twelvemonth ago, was brought forward.",
    ] {
        assert!(strict(&erg, text), "{text}");
    }
    assert!(!strict(&erg, "We saw yesterday the cats."));
}

#[test]
fn plural_family_names() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    for text in [
        "The Bennets were able to eat.",
        "The Gardiners stayed only one night.",
    ] {
        assert!(strict(&erg, text), "{text}");
    }
}

#[test]
fn sluicing_with_a_phrase() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    for text in [
        "They walked on, without knowing in what direction.",
        "I do not know with whom.",
        "Nobody knows which one.",
    ] {
        assert!(strict(&erg, text), "{text}");
    }
}

#[test]
fn transitive_meditate() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    for text in [
        "She meditated revenge.",
        "Not a syllable had ever reached her of Miss Darcy's meditated elopement.",
    ] {
        assert!(strict(&erg, text), "{text}");
    }
}

#[test]
fn phrasal_compounds() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    for text in [
        "I previously wanted allow-by-default.",
        "We use a pay-as-you-go model.",
        "We like one-size-fits-all.",
    ] {
        assert!(strict(&erg, text), "{text}");
    }
    // A compound the lexicon has is read as that entry, not as words
    // joined by dashes.
    let p = erg.parse("We chose a wait-and-see approach.").unwrap();
    let best = &p.readings[0];
    assert!(
        best.words.iter().any(|w| w.entry == "wait_and_see_a2"),
        "{:?}",
        best.words.iter().map(|w| &w.entry).collect::<Vec<_>>()
    );
}

#[test]
fn adjunct_between_commas_after_the_subject() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    for text in [
        "Kim, indeed, left.",
        "The committee, however, decided to wait.",
        "I think that the committee, in the end, decided to wait.",
    ] {
        assert!(strict(&erg, text), "{text}");
    }
    // A comma between a subject and its verb is not formal.
    assert!(!strict(&erg, "Kim, left."));
}

#[test]
fn amounts_with_a_magnitude_suffix() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    for text in [
        "Investors put £2bn into the funds.",
        "They raised $5m last year.",
        "They paid $300k.",
    ] {
        assert!(strict(&erg, text), "{text}");
    }
}
