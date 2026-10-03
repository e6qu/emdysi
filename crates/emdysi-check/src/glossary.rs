//! Glossaries in other formats: TBX (ISO 30042, the TermBase eXchange
//! format of terminology tools) and Vale vocabularies (`accept.txt` and
//! `reject.txt`), read into the glossary model of [`crate::terms`], and
//! written back as TBX or as emdysi's TOML.
//!
//! TBX is read in both layouts in use: TBX v3 (`conceptEntry`, `langSec`,
//! `termSec`) and the older TBX-Basic (`termEntry`, `langSet`, `tig` or
//! `ntig`). Only English language sections are used. The administrative
//! status (`preferredTerm-admn-sts`, `admittedTerm-admn-sts`,
//! `deprecatedTerm-admn-sts`, `supersededTerm-admn-sts`) and the part of
//! speech map onto a term's status and `pos`; a term without a status is
//! admitted.

use std::fmt::Write as _;

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::terms::{Concept, Pos, Status, Term};

fn local(name: &[u8]) -> String {
    let s = String::from_utf8_lossy(name);
    s.rsplit(':').next().unwrap_or("").to_string()
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn attr(e: &quick_xml::events::BytesStart, key: &str) -> Option<String> {
    e.attributes().flatten().find_map(|a| {
        (String::from_utf8_lossy(a.key.as_ref()) == key)
            .then(|| unescape(&String::from_utf8_lossy(&a.value)))
    })
}

fn status_of(s: &str) -> Option<Status> {
    match s.trim() {
        "preferredTerm-admn-sts" | "preferred" => Some(Status::Preferred),
        "admittedTerm-admn-sts" | "admitted" => Some(Status::Admitted),
        "deprecatedTerm-admn-sts" | "deprecated" => Some(Status::Deprecated),
        "supersededTerm-admn-sts" | "superseded" => Some(Status::Superseded),
        _ => None,
    }
}

fn pos_of(s: &str) -> Option<Pos> {
    match s.trim() {
        "noun" | "properNoun" => Some(Pos::Noun),
        "verb" => Some(Pos::Verb),
        "adjective" => Some(Pos::Adjective),
        "adverb" => Some(Pos::Adverb),
        _ => None,
    }
}

#[derive(Default)]
struct TermDraft {
    text: String,
    status: Option<Status>,
    pos: Option<Pos>,
}

/// What text is being collected.
enum Field {
    None,
    Term,
    Status,
    Pos,
    Definition,
}

/// Read the English concepts of a TBX document.
pub fn from_tbx(src: &str) -> Result<Vec<Concept>, String> {
    // Text is trimmed once collected: trimming each piece would drop the
    // spaces around an entity ("A &amp; B").
    let mut reader = Reader::from_str(src);
    let mut out = Vec::new();
    let mut concept: Option<Concept> = None;
    let mut english = true;
    let mut term: Option<TermDraft> = None;
    let mut field = Field::None;
    let mut text = String::new();
    loop {
        let ev = reader
            .read_event()
            .map_err(|e| format!("XML error at byte {}: {e}", reader.buffer_position()))?;
        match ev {
            Event::Start(e) => {
                let name = local(e.name().as_ref());
                match name.as_str() {
                    "conceptEntry" | "termEntry" => {
                        concept = Some(Concept {
                            id: attr(&e, "id")
                                .unwrap_or_else(|| format!("concept-{}", out.len() + 1)),
                            definition: String::new(),
                            terms: Vec::new(),
                        });
                    }
                    "langSec" | "langSet" => {
                        english =
                            attr(&e, "xml:lang").is_none_or(|l| l.to_lowercase().starts_with("en"));
                    }
                    "termSec" | "tig" | "ntig" if english => term = Some(TermDraft::default()),
                    "term" => field = Field::Term,
                    "termNote" => {
                        field = match attr(&e, "type").as_deref() {
                            Some("administrativeStatus") => Field::Status,
                            Some("partOfSpeech") => Field::Pos,
                            _ => Field::None,
                        }
                    }
                    "descrip" if english && attr(&e, "type").as_deref() == Some("definition") => {
                        field = Field::Definition;
                    }
                    _ => {}
                }
                text.clear();
            }
            Event::Text(t) => {
                text.push_str(&t.decode().map_err(|e| e.to_string())?);
            }
            Event::GeneralRef(r) => {
                let ch = r.resolve_char_ref().map_err(|e| e.to_string())?;
                match ch {
                    Some(c) => text.push(c),
                    None => {
                        let name = r.decode().map_err(|e| e.to_string())?;
                        text.push_str(match name.as_ref() {
                            "amp" => "&",
                            "lt" => "<",
                            "gt" => ">",
                            "quot" => "\"",
                            "apos" => "'",
                            _ => "",
                        });
                    }
                }
            }
            Event::CData(t) => text.push_str(&String::from_utf8_lossy(&t)),
            Event::End(e) => {
                let name = local(e.name().as_ref());
                let value = text.trim().to_string();
                match (&field, name.as_str()) {
                    (Field::Term, "term") => {
                        if let Some(t) = term.as_mut() {
                            t.text = value;
                        }
                    }
                    (Field::Status, "termNote") => {
                        if let Some(t) = term.as_mut() {
                            t.status = status_of(&value);
                        }
                    }
                    (Field::Pos, "termNote") => {
                        if let Some(t) = term.as_mut() {
                            t.pos = pos_of(&value);
                        }
                    }
                    (Field::Definition, "descrip") => {
                        if let Some(c) = concept.as_mut() {
                            if c.definition.is_empty() {
                                c.definition = value;
                            }
                        }
                    }
                    _ => {}
                }
                match name.as_str() {
                    "term" | "termNote" | "descrip" => field = Field::None,
                    "termSec" | "tig" | "ntig" => {
                        if let (Some(t), Some(c)) = (term.take(), concept.as_mut()) {
                            if !t.text.is_empty() {
                                c.terms.push(Term::new(
                                    &t.text,
                                    t.status.unwrap_or(Status::Admitted),
                                    t.pos,
                                    None,
                                ));
                            }
                        }
                    }
                    "langSec" | "langSet" => english = true,
                    "conceptEntry" | "termEntry" => {
                        if let Some(c) = concept.take() {
                            if !c.terms.is_empty() {
                                out.push(c);
                            }
                        }
                    }
                    _ => {}
                }
                text.clear();
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(out)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A TBX v3 document in the TBX-Basic dialect.
pub fn to_tbx(concepts: &[Concept]) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<tbx type=\"TBX-Basic\" style=\"dca\" xml:lang=\"en\" xmlns=\"urn:iso:std:iso:30042:ed-2\">\n");
    out.push_str("  <tbxHeader>\n    <fileDesc>\n      <sourceDesc>\n        <p>Exported by emdysi</p>\n      </sourceDesc>\n    </fileDesc>\n  </tbxHeader>\n");
    out.push_str("  <text>\n    <body>\n");
    for c in concepts {
        let _ = writeln!(out, "      <conceptEntry id=\"{}\">", esc(&c.id));
        if !c.definition.is_empty() {
            let _ = writeln!(
                out,
                "        <descrip type=\"definition\">{}</descrip>",
                esc(&c.definition)
            );
        }
        out.push_str("        <langSec xml:lang=\"en\">\n");
        for t in &c.terms {
            out.push_str("          <termSec>\n");
            let _ = writeln!(out, "            <term>{}</term>", esc(&t.text));
            if let Some(p) = t.pos {
                let _ = writeln!(
                    out,
                    "            <termNote type=\"partOfSpeech\">{}</termNote>",
                    p.name()
                );
            }
            let _ = writeln!(
                out,
                "            <termNote type=\"administrativeStatus\">{}Term-admn-sts</termNote>",
                t.status.name()
            );
            out.push_str("          </termSec>\n");
        }
        out.push_str("        </langSec>\n      </conceptEntry>\n");
    }
    out.push_str("    </body>\n  </text>\n</tbx>\n");
    out
}

fn toml_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A glossary file in emdysi's TOML (`[[concept]]` tables).
pub fn to_toml(concepts: &[Concept]) -> String {
    let mut out = String::new();
    for (i, c) in concepts.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str("[[concept]]\n");
        let _ = writeln!(out, "id = {}", toml_str(&c.id));
        if !c.definition.is_empty() {
            let _ = writeln!(out, "definition = {}", toml_str(&c.definition));
        }
        out.push_str("term = [\n");
        for t in &c.terms {
            let mut fields = vec![
                format!("text = {}", toml_str(&t.text)),
                format!("status = \"{}\"", t.status.name()),
            ];
            if let Some(p) = t.pos {
                fields.push(format!("pos = \"{}\"", p.name()));
            }
            let default_case = t.text.chars().any(char::is_uppercase);
            if t.exact_case != default_case {
                fields.push(format!(
                    "case = \"{}\"",
                    if t.exact_case { "exact" } else { "any" }
                ));
            }
            let _ = writeln!(out, "  {{ {} }},", fields.join(", "));
        }
        out.push_str("]\n");
    }
    out
}

/// A Vale vocabulary entry as a literal term: plain text, or a word whose
/// first letter is given in both cases (`[Pp]ython`). Other regular
/// expressions are not converted. Returns the text and whether its case
/// must match exactly.
fn vale_literal(line: &str) -> Option<(String, bool)> {
    let chars: Vec<char> = line.chars().collect();
    if chars.len() >= 4 && chars[0] == '[' && chars[3] == ']' {
        let (a, b) = (chars[1], chars[2]);
        if a.is_alphabetic() && a.to_lowercase().eq(b.to_lowercase()) && a != b {
            let rest: String = chars[4..].iter().collect();
            return vale_literal(&rest).map(|(r, _)| (format!("{}{r}", a.to_lowercase()), false));
        }
    }
    if line.chars().any(|c| "\\^$.|?*+()[]{}".contains(c)) {
        return None;
    }
    Some((line.to_string(), true))
}

/// Concepts from a Vale vocabulary: each `accept.txt` entry is a preferred
/// term (with Vale's case-sensitive matching), each `reject.txt` entry a
/// deprecated one with no replacement. Returns the concepts and the entries
/// that are regular expressions and were skipped.
pub fn from_vale_vocab(accept: &str, reject: &str) -> (Vec<Concept>, Vec<String>) {
    let mut out = Vec::new();
    let mut skipped = Vec::new();
    for (list, status) in [(accept, Status::Preferred), (reject, Status::Deprecated)] {
        for line in list.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            match vale_literal(line) {
                Some((text, exact)) => out.push(Concept {
                    id: format!(
                        "vale-{}-{}",
                        status.name(),
                        text.to_lowercase().replace(' ', "-")
                    ),
                    definition: String::new(),
                    terms: vec![Term::new(
                        &text,
                        status,
                        None,
                        Some(exact && status == Status::Preferred),
                    )],
                }),
                None => skipped.push(line.to_string()),
            }
        }
    }
    (out, skipped)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TBX3: &str = r#"<?xml version="1.0"?>
<tbx type="TBX-Basic" style="dca" xml:lang="en" xmlns="urn:iso:std:iso:30042:ed-2">
<text><body>
  <conceptEntry id="c1">
    <descrip type="definition">Authenticating to an account &amp; session.</descrip>
    <langSec xml:lang="en">
      <termSec><term>sign in</term>
        <termNote type="partOfSpeech">verb</termNote>
        <termNote type="administrativeStatus">preferredTerm-admn-sts</termNote></termSec>
      <termSec><term>log in</term>
        <termNote type="administrativeStatus">deprecatedTerm-admn-sts</termNote></termSec>
    </langSec>
    <langSec xml:lang="de">
      <termSec><term>anmelden</term></termSec>
    </langSec>
  </conceptEntry>
</body></text></tbx>"#;

    const TBX2: &str = r#"<martif type="TBX-Basic"><text><body>
  <termEntry id="t1"><langSet xml:lang="en-US">
    <tig><term>JavaScript</term><termNote type="administrativeStatus">admittedTerm-admn-sts</termNote></tig>
  </langSet></termEntry>
</body></text></martif>"#;

    #[test]
    fn reads_tbx_v3_and_basic() {
        let c = from_tbx(TBX3).unwrap();
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].id, "c1");
        assert_eq!(c[0].definition, "Authenticating to an account & session.");
        assert_eq!(c[0].terms.len(), 2, "German terms are skipped");
        assert_eq!(c[0].terms[0].status, Status::Preferred);
        assert_eq!(c[0].terms[0].pos, Some(Pos::Verb));
        assert_eq!(c[0].terms[1].status, Status::Deprecated);
        let c2 = from_tbx(TBX2).unwrap();
        assert_eq!(c2[0].terms[0].text, "JavaScript");
        assert!(c2[0].terms[0].exact_case);
        assert_eq!(c2[0].terms[0].status, Status::Admitted);
    }

    #[test]
    fn round_trips() {
        let c = from_tbx(TBX3).unwrap();
        let again = from_tbx(&to_tbx(&c)).unwrap();
        assert_eq!(to_toml(&c), to_toml(&again));
        let t = crate::rules::Pack::parse_glossary(&to_toml(&c)).unwrap();
        assert_eq!(t.concepts[0].terms[1].text, "log in");
    }

    #[test]
    fn reads_vale_vocab() {
        let (c, skipped) = from_vale_vocab("Kubernetes\n[Pp]ython\n(?i)foo.*\n", "utilize\n");
        assert_eq!(skipped, vec!["(?i)foo.*"]);
        assert_eq!(c.len(), 3);
        assert!(c[0].terms[0].exact_case);
        assert_eq!(c[1].terms[0].text, "python");
        assert!(!c[1].terms[0].exact_case);
        assert_eq!(c[2].terms[0].status, Status::Deprecated);
    }
}
