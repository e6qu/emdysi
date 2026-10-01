//! Helpers for reading [incr tsdb()] profiles in the examples.
#![allow(dead_code)]

use std::process::Command;

pub fn read_relation(dir: &str, name: &str) -> Vec<Vec<String>> {
    let path = format!("{dir}/{name}.gz");
    let out = Command::new("zcat").arg(&path).output().expect("zcat");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.split('@').map(unescape).collect())
        .collect()
}

/// Undo [incr tsdb()] field escaping (`\\`, `\s` for `@`, `\n`).
pub fn unescape(f: &str) -> String {
    let mut out = String::new();
    let mut chars = f.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('s') => out.push('@'),
                Some('n') => out.push('\n'),
                Some(d) => out.push(d),
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[derive(Debug)]
pub enum Sexp {
    Atom(String),
    List(Vec<Sexp>),
}

pub fn parse_sexp(s: &str) -> Option<Sexp> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    parse_at(&chars, &mut i)
}

pub fn parse_at(c: &[char], i: &mut usize) -> Option<Sexp> {
    while *i < c.len() && c[*i].is_whitespace() {
        *i += 1;
    }
    if *i >= c.len() {
        return None;
    }
    match c[*i] {
        '(' => {
            *i += 1;
            let mut items = Vec::new();
            loop {
                while *i < c.len() && c[*i].is_whitespace() {
                    *i += 1;
                }
                if *i >= c.len() {
                    return None;
                }
                if c[*i] == ')' {
                    *i += 1;
                    return Some(Sexp::List(items));
                }
                items.push(parse_at(c, i)?);
            }
        }
        '"' => {
            *i += 1;
            let mut s = String::new();
            while *i < c.len() && c[*i] != '"' {
                if c[*i] == '\\' {
                    *i += 1;
                }
                if *i < c.len() {
                    s.push(c[*i]);
                }
                *i += 1;
            }
            *i += 1;
            Some(Sexp::Atom(format!("\"{s}\"")))
        }
        _ => {
            let mut s = String::new();
            while *i < c.len() && !c[*i].is_whitespace() && c[*i] != '(' && c[*i] != ')' {
                s.push(c[*i]);
                *i += 1;
            }
            Some(Sexp::Atom(s))
        }
    }
}

/// Canonical skeleton `(name start end children...)`, dropping leaves.
pub fn skeleton(s: &Sexp, gold: bool) -> String {
    let Sexp::List(items) = s else {
        return String::new();
    };
    let atoms: Vec<&str> = items
        .iter()
        .map_while(|x| match x {
            Sexp::Atom(a) => Some(a.as_str()),
            _ => None,
        })
        .collect();
    // Gold: (id name score start end kids) or (root kid); ours: (name start end kids).
    let (name, span) = if gold {
        match atoms.len() {
            1 => (atoms[0], None),
            n if n >= 5 => (atoms[1], Some((atoms[3], atoms[4]))),
            _ => return String::new(),
        }
    } else {
        match atoms.len() {
            n if n >= 3 => (atoms[0], Some((atoms[1], atoms[2]))),
            _ => return String::new(),
        }
    };
    if name.starts_with('"') {
        return String::new();
    }
    let kids: Vec<String> = items[atoms.len()..]
        .iter()
        .map(|k| skeleton(k, gold))
        .filter(|k| !k.is_empty())
        .collect();
    match span {
        Some((a, b)) => format!(
            "({name} {a} {b}{}{})",
            if kids.is_empty() { "" } else { " " },
            kids.join(" ")
        ),
        None => kids.join(" "),
    }
}

/// An item of a profile: id, text, whether it is grammatical, and the
/// skeleton of its gold derivation, if any.
pub struct Item {
    pub id: String,
    pub text: String,
    pub wf: bool,
    pub gold: Option<String>,
    /// The gold reading's MRS (SimpleMRS), if recorded.
    pub mrs: Option<String>,
}

pub fn items(dir: &str) -> Vec<Item> {
    let parse_to_item: std::collections::HashMap<String, String> = read_relation(dir, "parse")
        .into_iter()
        .filter(|f| f.len() > 2)
        .map(|f| (f[0].clone(), f[2].clone()))
        .collect();
    let mut gold: std::collections::HashMap<String, String> = Default::default();
    let mut mrs: std::collections::HashMap<String, String> = Default::default();
    for f in read_relation(dir, "result") {
        if f.len() > 10 {
            if let (Some(item), Some(tree)) = (parse_to_item.get(&f[0]), parse_sexp(&f[10])) {
                gold.insert(item.clone(), skeleton(&tree, true));
                if let Some(m) = f.get(13).filter(|m| !m.is_empty()) {
                    mrs.insert(item.clone(), m.clone());
                }
            }
        }
    }
    read_relation(dir, "item")
        .into_iter()
        .filter(|f| f.len() > 10)
        .map(|f| Item {
            gold: gold.get(&f[0]).cloned(),
            mrs: mrs.get(&f[0]).cloned(),
            id: f[0].clone(),
            text: f[6].clone(),
            wf: f[10] == "1",
        })
        .collect()
}
