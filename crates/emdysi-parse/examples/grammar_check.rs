//! Load the grammar and report problems: load warnings (rules that could
//! not be built, ...), whether the named rules are present, and the lexical
//! entries of the named words.
//!
//! `cargo run --release -p emdysi-parse --example grammar_check -- [rule|word...]`

use emdysi_parse::{Erg, default_grammar_dir};

fn main() {
    let erg = Erg::load(&default_grammar_dir()).unwrap();
    for e in &erg.grammar.errors {
        println!("grammar error: {}: {}", e.what, e.msg);
    }
    for w in &erg.warnings {
        println!("warning: {w}");
    }
    // GLB=a,b prints the greatest lower bound of two types.
    if let Ok(pair) = std::env::var("GLB") {
        let (a, b) = pair.split_once(',').unwrap();
        let ts = &erg.grammar.ts;
        let (x, y) = (ts.hier.id(a).unwrap(), ts.hier.id(b).unwrap());
        println!("glb({a}, {b}) = {:?}", ts.glb(x, y).map(|t| ts.name(t)));
    }
    for a in std::env::args().skip(1) {
        if let Some(t) = erg.grammar.ts.hier.id(&a) {
            let d = erg.grammar.constraint(t);
            println!("type {a} ({} nodes)", d.len());
            if std::env::var_os("SHOW").is_some() {
                println!("{}", erg.grammar.display(d));
            }
        } else if erg.rules.iter().any(|r| r.name == a) {
            println!("rule {a}: present");
        } else if a.ends_with("_c") {
            println!("rule {a}: MISSING");
        } else {
            let entries: Vec<String> = erg
                .orth
                .iter()
                .filter(|(_, o)| o.eq_ignore_ascii_case(&a))
                .map(|(&i, _)| {
                    format!(
                        "{} ({})",
                        erg.grammar.instances[i].name,
                        erg.le_types.get(i).cloned().unwrap_or_default()
                    )
                })
                .collect();
            println!("word {a}: {}", entries.join(", "));
            if let Some(i) = erg.grammar.instance(&a) {
                let mut u = emdysi_hpsg::Unifier::new();
                match erg.grammar.expand(&i.body.clone(), &mut u) {
                    Ok(d) => println!("instance {a}: expands ({} nodes)", d.len()),
                    Err(e) => println!("instance {a}: does not expand: {e:?}"),
                }
            }
        }
    }
}
