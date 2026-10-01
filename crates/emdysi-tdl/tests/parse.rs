use emdysi_tdl::*;

fn one(src: &str) -> Definition {
    let mut st = parse_str(src, "test").unwrap();
    assert_eq!(st.len(), 1, "{st:?}");
    match st.pop().unwrap() {
        Statement::Def(d) => d,
        s => panic!("not a definition: {s:?}"),
    }
}

#[test]
fn type_with_avm_paths_and_corefs() {
    let d = one(
        "foo := bar & baz &\n [ A.B #x, C < #x, ... >, D <! !>, E \"s\\\"q\", F ^[0-9]+$ ]. ; c",
    );
    assert_eq!(d.name, "foo");
    assert_eq!(d.op, DefOp::Define);
    let t = &d.body.0;
    assert_eq!(t[0], Term::Type("bar".into()));
    let Term::Avm(fvs) = &t[2] else { panic!() };
    assert_eq!(fvs[0].path, vec!["A", "B"]);
    assert_eq!(fvs[0].value.0, vec![Term::Coref("x".into())]);
    assert!(matches!(&fvs[1].value.0[0], Term::List { open: true, items, .. } if items.len() == 1));
    assert_eq!(fvs[2].value.0, vec![Term::DiffList(vec![])]);
    assert_eq!(fvs[3].value.0, vec![Term::Str("s\"q".into())]);
    assert_eq!(fvs[4].value.0, vec![Term::Regex("^[0-9]+$".into())]);
}

#[test]
fn docstrings_and_dotted_list() {
    let d = one("t := u\n\"\"\"\nSome doc\n\"\"\"\n.");
    assert_eq!(d.docstrings, vec!["Some doc"]);
    let d = one("t := [ L < a . b > ].");
    let Term::Avm(fvs) = &d.body.0[0] else {
        panic!()
    };
    assert!(matches!(
        &fvs[0].value.0[0],
        Term::List { tail: Some(_), .. }
    ));
}

#[test]
fn affixes_and_letter_sets() {
    let st = parse_str(
        "%(letter-set (!s ab\\(c))\nn_pl := \n%suffix (!s !ss) (* ed) (es eses)\n\"\"\"d\"\"\"\nrule & [ X + ].",
        "t",
    )
    .unwrap();
    assert_eq!(
        st[0],
        Statement::LetterSet {
            var: 's',
            chars: vec!['a', 'b', '(', 'c'],
            wild: false
        }
    );
    let Statement::Def(d) = &st[1] else { panic!() };
    let a = d.affix.as_ref().unwrap();
    assert!(!a.prefix);
    assert_eq!(
        a.pairs[0],
        (
            vec![PatElem::Var('s')],
            vec![PatElem::Var('s'), PatElem::Char('s')]
        )
    );
    assert_eq!(
        a.pairs[1],
        (vec![], vec![PatElem::Char('e'), PatElem::Char('d')])
    );
    assert_eq!(d.docstrings, vec!["d"]);
}

#[test]
fn block_comments_and_directives() {
    let st = parse_str(
        "#| comment\n x := y. |#\n:begin :instance :status lex-entry.\n:include \"lexicon\".\n:end :instance.\n",
        "t",
    )
    .unwrap();
    assert_eq!(
        st,
        vec![
            Statement::Begin {
                kind: "instance".into(),
                status: Some("lex-entry".into())
            },
            Statement::Include("lexicon".into()),
            Statement::End {
                kind: "instance".into()
            },
        ]
    );
}

#[test]
fn addendum() {
    let d = one("phrase :+ no_inner_delim_phrase.");
    assert_eq!(d.op, DefOp::Addendum);
}
