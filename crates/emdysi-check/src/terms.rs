//! Terminology: a project glossary of concepts and their terms, acronyms
//! defined on first use, one spelling per term within a document, and
//! words or concept names the writer coined instead of using established
//! ones.
//!
//! The glossary follows the TBX-Basic model: a concept has terms, each
//! with a status (`preferred`, `admitted`, `deprecated` or `superseded`),
//! optionally a part of speech, and a case policy. Acronym definitions are
//! found by the Schwartz–Hearst alignment of a short form with the words
//! before it (Schwartz and Hearst 2003, "A simple algorithm for
//! identifying abbreviation definitions in biomedical text").

use std::collections::HashMap;

use emdysi_parse::Erg;
use emdysi_text::blocks::BlockKind;
use fancy_regex::Regex;

use crate::structure::Hit;
use crate::toml::{Table, Value};
use crate::{Analysis, Sentence};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Preferred,
    Admitted,
    Deprecated,
    Superseded,
}

impl Status {
    fn parse(s: &str) -> Option<Status> {
        match s {
            "preferred" => Some(Status::Preferred),
            "admitted" => Some(Status::Admitted),
            "deprecated" => Some(Status::Deprecated),
            "superseded" => Some(Status::Superseded),
            _ => None,
        }
    }

    pub fn allowed(self) -> bool {
        matches!(self, Status::Preferred | Status::Admitted)
    }
}

/// Part of speech of a term, as the grammar's lexical types give it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pos {
    Noun,
    Verb,
    Adjective,
    Adverb,
}

impl Pos {
    fn parse(s: &str) -> Option<Pos> {
        match s {
            "noun" => Some(Pos::Noun),
            "verb" => Some(Pos::Verb),
            "adjective" | "adj" => Some(Pos::Adjective),
            "adverb" | "adv" => Some(Pos::Adverb),
            _ => None,
        }
    }

    /// Whether an ERG lexical type is of this part of speech.
    pub fn matches(self, le_type: &str) -> bool {
        let prefix = match self {
            Pos::Noun => "n_",
            Pos::Verb => "v_",
            Pos::Adjective => "aj_",
            Pos::Adverb => "av_",
        };
        le_type.starts_with(prefix)
    }
}

#[derive(Debug, Clone)]
pub struct Term {
    pub text: String,
    /// Lower-case words of the term.
    pub words: Vec<String>,
    pub status: Status,
    pub pos: Option<Pos>,
    /// Must be written exactly as `text` (default: when `text` has a
    /// capital letter).
    pub exact_case: bool,
}

#[derive(Debug, Clone)]
pub struct Concept {
    pub id: String,
    pub definition: String,
    pub terms: Vec<Term>,
}

impl Concept {
    pub fn preferred(&self) -> Option<&Term> {
        self.terms
            .iter()
            .find(|t| t.status == Status::Preferred)
            .or_else(|| self.terms.iter().find(|t| t.status.allowed()))
    }
}

/// Parse the `[[concept]]` tables of a pack.
pub fn parse_concepts(t: &Table) -> Result<Vec<Concept>, String> {
    let mut out = Vec::new();
    for (i, c) in t
        .get("concept")
        .and_then(Value::as_array)
        .unwrap_or(&[])
        .iter()
        .enumerate()
    {
        let c = c
            .as_table()
            .ok_or_else(|| format!("concept {} is not a table", i + 1))?;
        let id = c
            .get("id")
            .and_then(Value::as_str)
            .map(String::from)
            .unwrap_or_else(|| format!("concept-{}", i + 1));
        let mut terms = Vec::new();
        for term in c.get("term").and_then(Value::as_array).unwrap_or(&[]) {
            let term = term
                .as_table()
                .ok_or_else(|| format!("{id}: a term is not a table"))?;
            let text = term
                .get("text")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("{id}: a term has no `text`"))?
                .to_string();
            let status = match term.get("status").and_then(Value::as_str) {
                None => Status::Preferred,
                Some(s) => Status::parse(s).ok_or_else(|| format!("{id}: unknown status {s:?}"))?,
            };
            let pos = match term.get("pos").and_then(Value::as_str) {
                None => None,
                Some(p) => Some(Pos::parse(p).ok_or_else(|| format!("{id}: unknown pos {p:?}"))?),
            };
            let exact_case = match term.get("case").and_then(Value::as_str) {
                None => text.chars().any(char::is_uppercase),
                Some("exact") => true,
                Some("any") => false,
                Some(o) => {
                    return Err(format!(
                        "{id}: `case` must be \"exact\" or \"any\", not {o:?}"
                    ));
                }
            };
            terms.push(Term {
                words: split_words(&text),
                text,
                status,
                pos,
                exact_case,
            });
        }
        if terms.is_empty() {
            return Err(format!("{id}: a concept needs at least one `term`"));
        }
        out.push(Concept {
            id,
            definition: c
                .get("definition")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            terms,
        });
    }
    Ok(out)
}

/// Lower-case words of a term, split at spaces (hyphens are kept).
fn split_words(s: &str) -> Vec<String> {
    s.split_whitespace().map(|w| w.to_lowercase()).collect()
}

/// The concepts of all loaded packs.
#[derive(Debug, Clone, Default)]
pub struct Glossary {
    pub concepts: Vec<Concept>,
}

impl Glossary {
    /// Whether a word (any case) belongs to an allowed term.
    pub fn knows_word(&self, w: &str) -> bool {
        let w = w.to_lowercase();
        self.concepts.iter().any(|c| {
            c.terms.iter().any(|t| {
                t.status.allowed()
                    && t.words
                        .iter()
                        .any(|x| *x == w || x.split('-').any(|p| p == w))
            })
        })
    }

    /// Character spans of allowed terms of two or more words in a sentence.
    pub fn term_spans(&self, s: &Sentence) -> Vec<(usize, usize)> {
        let toks = word_tokens(s);
        let mut out = Vec::new();
        for c in &self.concepts {
            for t in c.terms.iter().filter(|t| t.status.allowed()) {
                for (from, to, _) in find_seq(&toks, &t.words) {
                    out.push((from, to));
                }
            }
        }
        out
    }
}

/// Word tokens of a sentence (lower case) with character spans, outside
/// inline code. Hyphenated words are kept whole.
fn word_tokens(s: &Sentence) -> Vec<(usize, usize, String)> {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let re =
        RE.get_or_init(|| Regex::new(r"[\p{L}\p{N}][\p{L}\p{N}'’]*(?:-[\p{L}\p{N}]+)*").unwrap());
    let masked: Vec<char> = s.text.chars().collect();
    let mut out = Vec::new();
    for m in re.find_iter(&s.original).flatten() {
        let from = s.original[..m.start()].chars().count();
        let len = m.as_str().chars().count();
        let to = from + len;
        if masked
            .get(from..to)
            .is_some_and(|c| c.iter().all(|&c| c == 'x'))
            && !m.as_str().chars().all(|c| c == 'x')
        {
            continue; // inline code
        }
        out.push((from, to, m.as_str().to_string()));
    }
    out
}

/// Occurrences of a word sequence (lower case) in word tokens.
fn find_seq(toks: &[(usize, usize, String)], words: &[String]) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    if words.is_empty() || words.len() > toks.len() {
        return out;
    }
    for i in 0..=toks.len() - words.len() {
        if (0..words.len()).all(|k| toks[i + k].2.to_lowercase() == words[k]) {
            out.push((toks[i].0, toks[i + words.len() - 1].1, i));
        }
    }
    out
}

/// Give `rep` the capitalization of `like`'s first letter.
fn match_initial(like: &str, rep: &str) -> String {
    if like.chars().next().is_some_and(char::is_uppercase)
        && !rep.chars().next().is_some_and(char::is_uppercase)
    {
        let mut c = rep.chars();
        c.next()
            .map(|f| f.to_uppercase().chain(c).collect())
            .unwrap_or_default()
    } else {
        rep.to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlossaryCheck {
    /// A deprecated or superseded term: use the concept's preferred term.
    Deprecated,
    /// An allowed term written with the wrong capitalization.
    Casing,
}

/// Words of the best reading that overlap a character span.
fn words_in(s: &Sentence, from: usize, to: usize) -> Vec<&emdysi_parse::Word> {
    s.best()
        .map(|r| {
            r.words
                .iter()
                .filter(|w| w.from < to && from < w.to)
                .collect()
        })
        .unwrap_or_default()
}

pub fn run_glossary(check: GlossaryCheck, g: &Glossary, a: &Analysis) -> Vec<Hit> {
    let mut out = Vec::new();
    for (si, s) in a.sentences.iter().enumerate() {
        let toks = word_tokens(s);
        // Lemmas of the best reading, by character span, for inflected
        // forms ("logged in" for "log in").
        let lemmas: Vec<(usize, usize, String)> = toks
            .iter()
            .map(|(f, t, w)| {
                let lemma = words_in(s, *f, *t)
                    .iter()
                    .find(|x| x.from == *f && x.to == *t && !x.lemma.contains(' '))
                    .map(|x| x.lemma.to_lowercase())
                    .unwrap_or_else(|| w.to_lowercase());
                (*f, *t, lemma)
            })
            .collect();
        for c in &g.concepts {
            for t in &c.terms {
                match check {
                    GlossaryCheck::Deprecated if !t.status.allowed() => {
                        let Some(pref) = c.preferred() else { continue };
                        let mut found = find_seq(&toks, &t.words);
                        for m in find_seq(&lemmas, &t.words) {
                            if !found.iter().any(|f| f.0 == m.0) {
                                found.push(m);
                            }
                        }
                        for (from, to, _) in found {
                            if let Some(pos) = t.pos {
                                let ws = words_in(s, from, to);
                                if !ws.is_empty() && !ws.iter().any(|w| pos.matches(&w.le_type)) {
                                    continue;
                                }
                            }
                            let mut h = Hit::at(a, si, from, to)
                                .var("preferred", &pref.text)
                                .var("concept", &c.id);
                            // Fix only the uninflected form; inflected
                            // forms get a suggestion.
                            if h.text.to_lowercase() == t.words.join(" ") {
                                h.replacement = Some(match_initial(&h.text, &pref.text));
                            } else {
                                h.suggestions = vec![pref.text.clone()];
                            }
                            out.push(h);
                        }
                    }
                    GlossaryCheck::Casing if t.status.allowed() && t.exact_case => {
                        for (from, to, _) in find_seq(&toks, &t.words) {
                            let h = Hit::at(a, si, from, to);
                            let initial = from == first_word_start(s)
                                && match_initial(&h.text, &t.text) == h.text;
                            if h.text != t.text && !initial {
                                let rep = t.text.clone();
                                let mut h = h.var("preferred", &rep);
                                h.replacement = Some(rep);
                                out.push(h);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    out
}

fn first_word_start(s: &Sentence) -> usize {
    s.original
        .chars()
        .position(char::is_alphanumeric)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------
// Acronyms

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcronymCheck {
    /// Used but never defined.
    Undefined,
    /// Used before its definition.
    DefinedAfterUse,
    /// Defined but not used again.
    UsedOnce,
    /// Defined twice with different long forms.
    Redefined,
    /// First used in a heading.
    FirstUseInHeading,
}

/// Whether a word looks like an acronym or initialism: two or more capital
/// letters making up at least half of the letters, no run of three
/// lower-case letters (IaaS, PhD, API; not GitHub or OpenAI). A plural `s` is stripped.
pub fn acronym(w: &str) -> Option<&str> {
    let core = match w.strip_suffix('s') {
        Some(c)
            if c.len() >= 2
                && c.chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) =>
        {
            c
        }
        _ => w,
    };
    let n = core.chars().count();
    if !(2..=8).contains(&n)
        || !core.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        || !core.chars().all(|c| c.is_ascii_alphanumeric())
        || core.chars().filter(char::is_ascii_uppercase).count() < 2
        || 2 * core.chars().filter(char::is_ascii_uppercase).count()
            < core.chars().filter(char::is_ascii_alphabetic).count()
    {
        return None;
    }
    let mut lower_run = 0;
    for c in core.chars() {
        lower_run = if c.is_ascii_lowercase() {
            lower_run + 1
        } else {
            0
        };
        if lower_run >= 3 {
            return None;
        }
    }
    // Roman numerals (World War II, Henry VIII).
    if core.chars().all(|c| matches!(c, 'I' | 'V' | 'X')) {
        return None;
    }
    // Words in capitals for emphasis (NOT, MUST, NOTE), unless the
    // capitalized form is itself a listed word.
    let all_caps = core.chars().all(|c| !c.is_ascii_lowercase());
    if all_caps
        && crate::dict::words().contains_key(&core.to_lowercase())
        && !crate::dict::words().contains_key(core)
    {
        return None;
    }
    Some(core)
}

/// The long form that a short form abbreviates, ending the words before
/// it: Schwartz–Hearst alignment of the short form's letters, from the
/// end, with the first letter at the start of a word.
pub fn long_form(short: &str, before: &str) -> Option<String> {
    let s: Vec<char> = short.to_lowercase().chars().collect();
    let l: Vec<char> = before.chars().collect();
    let lower: Vec<char> = before.to_lowercase().chars().collect();
    let (mut si, mut li) = (s.len() as isize - 1, l.len() as isize - 1);
    while si >= 0 {
        let c = s[si as usize];
        if !c.is_alphanumeric() {
            si -= 1;
            continue;
        }
        while li >= 0
            && (lower[li as usize] != c
                || (si == 0 && li > 0 && l[li as usize - 1].is_alphanumeric()))
        {
            li -= 1;
        }
        if li < 0 {
            return None;
        }
        li -= 1;
        si -= 1;
    }
    let start = l[..(li + 1) as usize]
        .iter()
        .rposition(|c| c.is_whitespace())
        .map_or(0, |p| p + 1);
    let lf: String = l[start..].iter().collect();
    let lf = lf.trim().to_string();
    // The long form must have at least as many words as are needed and
    // must not just repeat the short form.
    if lf.is_empty() || lf.eq_ignore_ascii_case(short) {
        return None;
    }
    Some(lf)
}

#[derive(Debug, Clone)]
struct Use {
    sentence: usize,
    from: usize,
    to: usize,
    heading: bool,
    /// The long form, if this occurrence is a definition.
    defines: Option<String>,
}

fn acronym_uses(a: &Analysis) -> HashMap<String, Vec<Use>> {
    static PAREN: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    static STANDS: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let paren = PAREN.get_or_init(|| Regex::new(r"\(([^()]{1,80})\)").unwrap());
    let stands = STANDS.get_or_init(|| {
        Regex::new(r"^\s*,?\s*(?:stands for|is short for|is an abbreviation (?:for|of)|is an acronym for)\s+([^,.;:]+)").unwrap()
    });
    let mut out: HashMap<String, Vec<Use>> = HashMap::new();
    for (si, s) in a.sentences.iter().enumerate() {
        let heading = matches!(a.blocks[s.block].kind, BlockKind::Heading(_));
        let toks = word_tokens(s);
        let chars: Vec<char> = s.original.chars().collect();
        let parens: Vec<(usize, usize, String)> = paren
            .captures_iter(&s.original)
            .flatten()
            .map(|c| {
                let m = c.get(0).unwrap();
                let from = s.original[..m.start()].chars().count();
                (
                    from,
                    from + m.as_str().chars().count(),
                    c.get(1).unwrap().as_str().trim().to_string(),
                )
            })
            .collect();
        for (k, (from, to, w)) in toks.iter().enumerate() {
            let Some(sf) = acronym(w) else { continue };
            // Units and standards: "5 GB", "RFC 9110".
            let next_digit = toks
                .get(k + 1)
                .is_some_and(|n| n.2.chars().all(|c| c.is_ascii_digit()) && n.0 == to + 1);
            let prev_digit = k > 0 && toks[k - 1].2.chars().all(|c| c.is_ascii_digit());
            if next_digit || prev_digit {
                continue;
            }
            let mut defines = None;
            // "long form (SF)"
            if let Some(p) = parens.iter().find(|p| p.0 + 1 == *from && p.2 == *w) {
                let before: String = chars[..p.0].iter().collect();
                let before = before.trim_end();
                let n = sf.chars().count();
                let window = (n + 5).min(2 * n);
                let tail: Vec<&str> = before.split_whitespace().collect();
                let tail = tail[tail.len().saturating_sub(window)..].join(" ");
                defines = long_form(sf, &tail);
            }
            // "SF (long form)"
            if defines.is_none() {
                if let Some(p) = parens
                    .iter()
                    .find(|p| p.0 == *to + 1 || (p.0 == *to && chars.get(*to) == Some(&'(')))
                {
                    if p.2.split_whitespace().count() >= 2 && acronym(&p.2).is_none() {
                        defines = long_form(sf, &p.2).map(|_| p.2.clone());
                    }
                }
            }
            // "SF stands for long form"
            if defines.is_none() {
                let rest: String = chars[*to..].iter().collect();
                if let Ok(Some(c)) = stands.captures(&rest) {
                    defines = Some(c.get(1).unwrap().as_str().trim().to_string());
                }
            }
            out.entry(sf.to_string()).or_default().push(Use {
                sentence: si,
                from: *from,
                to: *to,
                heading,
                defines,
            });
        }
    }
    out
}

/// Whether an acronym counts as known without a definition: listed in the
/// rule, in the glossary, or a word list entry at least as common as
/// `known_tier`.
fn known_acronym(sf: &str, known_tier: u8, known: &[String], g: &Glossary) -> bool {
    known.iter().any(|k| k == sf)
        || g.concepts
            .iter()
            .any(|c| c.terms.iter().any(|t| t.status.allowed() && t.text == sf))
        || crate::dict::words()
            .get(sf)
            .is_some_and(|&t| t <= known_tier)
}

pub fn run_acronyms(
    check: AcronymCheck,
    a: &Analysis,
    known_tier: u8,
    known: &[String],
    g: &Glossary,
) -> Vec<Hit> {
    let mut out = Vec::new();
    let mut uses: Vec<(String, Vec<Use>)> = acronym_uses(a).into_iter().collect();
    uses.sort_by_key(|(_, u)| (u[0].sentence, u[0].from));
    for (sf, us) in uses {
        let is_known = known_acronym(&sf, known_tier, known, g);
        let defs: Vec<&Use> = us.iter().filter(|u| u.defines.is_some()).collect();
        let first_body = us.iter().find(|u| !u.heading);
        let hit = |u: &Use| Hit::at(a, u.sentence, u.from, u.to).var("acronym", &sf);
        match check {
            AcronymCheck::Undefined => {
                if !is_known && defs.is_empty() {
                    out.push(hit(first_body.unwrap_or(&us[0])));
                }
            }
            AcronymCheck::DefinedAfterUse => {
                if let (false, Some(d), Some(f)) = (is_known, defs.first(), first_body) {
                    if (f.sentence, f.from) < (d.sentence, d.from) {
                        let line = a.line_col(a.source_range(d.sentence, d.from, d.to).start).0;
                        out.push(hit(f).var("line", line));
                    }
                }
            }
            AcronymCheck::UsedOnce => {
                if defs.len() == 1 && us.len() == 1 {
                    let d = defs[0];
                    out.push(hit(d).var("long", d.defines.clone().unwrap_or_default()));
                }
            }
            AcronymCheck::Redefined => {
                let norm = |s: &str| s.to_lowercase().replace('-', " ");
                if let Some(first) = defs.first() {
                    let lf = norm(first.defines.as_deref().unwrap_or(""));
                    for d in defs.iter().skip(1) {
                        if norm(d.defines.as_deref().unwrap_or("")) != lf {
                            out.push(
                                hit(d)
                                    .var("long", d.defines.clone().unwrap_or_default())
                                    .var("first", first.defines.clone().unwrap_or_default()),
                            );
                        }
                    }
                }
            }
            AcronymCheck::FirstUseInHeading => {
                if !is_known && us[0].heading {
                    out.push(hit(&us[0]));
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------
// One spelling per term

/// Spellings of one term that differ only in hyphens, spaces or case
/// (e-mail / email, data set / dataset, front-end / frontend) used in one
/// document: the less used spellings are fixed to the most used one. When
/// the grammar gives the spellings different parts of speech ("set up" the
/// verb, "setup" the noun), they are different words and left alone.
const FUNCTION_WORDS: &[&str] = &[
    "a", "an", "the", "every", "any", "some", "no", "each", "all", "may", "can", "in", "on", "at",
    "out", "to", "with", "for", "be", "one", "body", "thing", "where", "how", "what", "ever",
    "self", "selves", "over", "under", "up", "down", "off", "way", "time", "day", "together",
    "never", "less", "more", "most", "much", "already", "ready", "so", "not",
];

/// An occurrence of a spelling: sentence, character span, text and the
/// part of speech (first letter of the grammar's lexical type).
type Spelling = (usize, usize, usize, String, Option<char>);

pub fn run_variants(a: &Analysis, min_length: usize, g: &Glossary) -> Vec<Hit> {
    // key -> occurrences (sentence, from, to, text, pos)
    let mut found: HashMap<String, Vec<Spelling>> = HashMap::new();
    let pos_of = |s: &Sentence, from: usize, to: usize| -> Option<char> {
        let ws = words_in(s, from, to);
        let head = ws
            .iter()
            .rev()
            .find(|w| w.surface.chars().any(char::is_alphabetic))?;
        head.le_type.chars().next()
    };
    for (si, s) in a.sentences.iter().enumerate() {
        if matches!(a.blocks[s.block].kind, BlockKind::Heading(_)) {
            continue;
        }
        let toks = word_tokens(s);
        let plain = |w: &str| w.chars().all(|c| c.is_alphabetic() || c == '-');
        for (k, (from, to, w)) in toks.iter().enumerate() {
            if !plain(w) || w.chars().filter(|c| c.is_uppercase()).count() > 1 {
                continue;
            }
            let key: String = w.to_lowercase().replace('-', "");
            if key.chars().count() >= min_length {
                found.entry(key).or_default().push((
                    si,
                    *from,
                    *to,
                    w.clone(),
                    pos_of(s, *from, *to),
                ));
            }
            // Two words written apart: "data set" (but not "every one",
            // "some time", "in to": function words join other words
            // differently).
            if let Some((nf, nt, nw)) = toks.get(k + 1) {
                if *nf == to + 1
                    && !FUNCTION_WORDS.contains(&w.to_lowercase().as_str())
                    && !FUNCTION_WORDS.contains(&nw.to_lowercase().as_str())
                    && plain(nw)
                    && !w.contains('-')
                    && !nw.contains('-')
                    && s.original.chars().nth(*to) == Some(' ')
                {
                    let key = format!("{}{}", w.to_lowercase(), nw.to_lowercase());
                    if key.chars().count() >= min_length {
                        found.entry(key).or_default().push((
                            si,
                            *from,
                            *nt,
                            format!("{w} {nw}"),
                            pos_of(s, *from, *nt),
                        ));
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    let mut keys: Vec<&String> = found.keys().collect();
    keys.sort();
    for key in keys {
        let occ = &found[key];
        // Spellings, ignoring case, in order of first use.
        let mut spellings: Vec<(String, usize, Option<char>)> = Vec::new();
        for o in occ {
            let low = o.3.to_lowercase();
            match spellings.iter_mut().find(|x| x.0 == low) {
                Some(x) => x.1 += 1,
                None => spellings.push((low, 1, o.4)),
            }
        }
        // Two-word spellings only count when some one-word or hyphenated
        // spelling exists too; otherwise "data set" alone is fine.
        if spellings.len() < 2 || spellings.iter().all(|x| x.0.contains(' ')) {
            continue;
        }
        if spellings.iter().any(|x| x.2.is_none()) {
            continue;
        }
        let pos0 = spellings[0].2;
        if spellings.iter().any(|x| x.2 != pos0) {
            continue;
        }
        // A spelling the glossary prefers wins; else the most used, then
        // the first used.
        let winner = spellings
            .iter()
            .find(|x| {
                g.concepts.iter().any(|c| {
                    c.terms
                        .iter()
                        .any(|t| t.status == Status::Preferred && t.text.to_lowercase() == x.0)
                })
            })
            .or_else(|| {
                let max = spellings.iter().map(|x| x.1).max().unwrap();
                spellings.iter().find(|x| x.1 == max)
            })
            .unwrap()
            .0
            .clone();
        for o in occ {
            if o.3.to_lowercase() == winner {
                continue;
            }
            let rep = match_initial(&o.3, &winner);
            let mut h = Hit::at(a, o.0, o.1, o.2).var("preferred", &rep);
            h.replacement = Some(rep);
            out.push(h);
        }
    }
    out
}

// ---------------------------------------------------------------------
// Coined words and concept names

/// Productive suffixes and prefixes, longest first, with what to add back
/// to get the base ("-ification" -> "-ify" is approximated by trying the
/// bare stem and the stem plus "e" or "y").
const SUFFIXES: &[&str] = &[
    "ification",
    "ization",
    "isation",
    "ability",
    "ibility",
    "fulness",
    "ishness",
    "ization",
    "ifying",
    "ified",
    "ifies",
    "ness",
    "ment",
    "less",
    "ful",
    "ify",
    "ize",
    "ise",
    "ism",
    "ist",
    "ity",
    "ish",
    "ish",
    "able",
    "ible",
    "ive",
    "ial",
    "al",
    "ic",
    "ly",
    "er",
    "ed",
    "ing",
    "s",
];
const PREFIXES: &[&str] = &[
    "hyper", "meta", "omni", "ultra", "mega", "super", "micro", "macro", "proto", "pseudo",
    "trans", "inter", "multi", "poly", "post", "pre", "re", "un", "de", "non", "co",
];

/// If `w` is not a known word but is a known word plus a productive affix,
/// the parts: (base, affix).
pub fn novel_derivation(erg: &Erg, w: &str) -> Option<(String, String)> {
    let w = w.to_lowercase();
    let known =
        |p: &str| p.chars().count() >= 3 && (crate::dict::tier(p).is_some() || erg.known_word(p));
    if w.chars().count() < 6 || !w.chars().all(char::is_alphabetic) || known(&w) {
        return None;
    }
    for suf in SUFFIXES {
        if let Some(stem) = w.strip_suffix(suf) {
            for base in [
                stem.to_string(),
                format!("{stem}e"),
                format!("{stem}y"),
                stem.strip_suffix('i')
                    .map(|s| format!("{s}y"))
                    .unwrap_or_default(),
            ] {
                if known(&base) {
                    return Some((base, format!("-{suf}")));
                }
            }
            // Two suffixes: "agentification" = agent + -ific + -ation.
            if let Some((b, inner)) = novel_derivation_inner(&known, stem) {
                return Some((b, format!("-{inner}{suf}")));
            }
        }
    }
    for pre in PREFIXES {
        if let Some(rest) = w.strip_prefix(pre) {
            let rest = rest.trim_start_matches('-');
            if known(rest) {
                return Some((rest.to_string(), format!("{pre}-")));
            }
        }
    }
    None
}

fn novel_derivation_inner(known: &dyn Fn(&str) -> bool, stem: &str) -> Option<(String, String)> {
    for suf in ["ific", "ic", "al", "iz", "is", "ation"] {
        if let Some(b) = stem.strip_suffix(suf) {
            if known(b) {
                return Some((b.to_string(), suf.to_string()));
            }
        }
    }
    None
}

/// Whether a sentence defines a word: "X is a ...", "we call this X",
/// "X (a ...)", "X means ...", "the term X", or X in quotation marks.
fn defines(s: &str, word: &str) -> bool {
    let w = fancy_regex::escape(word);
    let pats = [
        format!(
            r"(?i)\b{w}\b\W{{0,3}}\s+(?:is|are|means|refers to|denotes)\s+(?:a|an|the|what|when|how)\b"
        ),
        format!(
            r"(?i)\b(?:call|calls|called|term|termed|name|named|dub|dubbed|coin|coined)\b(?:\s+\w+){{0,3}}\s+\W?{w}\b"
        ),
        format!(r"(?i)\bthe (?:term|word|phrase|name)\s+\W?{w}\b"),
        format!(r#"(?i)["“‘']{w}["”’']"#),
        format!(r"(?i)\b{w}\s*\((?:a|an|the|i\.e\.)\s"),
    ];
    pats.iter()
        .any(|p| Regex::new(p).is_ok_and(|re| re.is_match(s).unwrap_or(false)))
}

/// Words the writer coined from a known word and an affix
/// ("promptability", "agentification"), not defined in the document or
/// the glossary. Only the first use is reported, with the count.
pub fn run_coined_words(erg: &Erg, a: &Analysis, g: &Glossary, ignore: &[String]) -> Vec<Hit> {
    let mut seen: HashMap<String, (usize, usize, usize, usize, String, String)> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    let mut defined: Vec<String> = Vec::new();
    for (si, s) in a.sentences.iter().enumerate() {
        for (from, to, w) in word_tokens(s) {
            let low = w.to_lowercase();
            if !w.chars().all(char::is_lowercase) || ignore.contains(&low) || g.knows_word(&low) {
                continue;
            }
            if let Some(e) = seen.get_mut(&low) {
                e.3 += 1;
                continue;
            }
            let Some((base, affix)) = novel_derivation(erg, &low) else {
                continue;
            };
            if defines(&s.original, &w) {
                defined.push(low.clone());
            }
            order.push(low.clone());
            seen.insert(low, (si, from, to, 1, base, affix));
        }
    }
    order
        .into_iter()
        .filter(|w| !defined.contains(w))
        .map(|w| {
            let (si, from, to, n, base, affix) = seen[&w].clone();
            Hit::at(a, si, from, to)
                .var("base", base)
                .var("affix", affix)
                .var("count", n)
        })
        .collect()
}

/// Capitalized concept names made of common words with a framework-like
/// head ("the Clarity Loop", "the Trust Tax"), not defined, linked or in
/// the glossary.
pub fn run_concept_names(
    a: &Analysis,
    heads: &[String],
    g: &Glossary,
    except: &[String],
) -> Vec<Hit> {
    if heads.is_empty() {
        return Vec::new();
    }
    let alt = heads
        .iter()
        .map(|h| fancy_regex::escape(h))
        .collect::<Vec<_>>()
        .join("|");
    let Ok(re) = Regex::new(&format!(
        r"\b(?:[Tt]he|[Oo]ur|[Mm]y|[Aa]n?|[Tt]his|[Tt]hat|[Ww]hat I call)\s+((?:[A-Z][a-z]+(?:-[A-Z]?[a-z]+)?\s+){{1,3}}(?:{alt}))\b"
    )) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for (si, s) in a.sentences.iter().enumerate() {
        if matches!(a.blocks[s.block].kind, BlockKind::Heading(_)) {
            continue;
        }
        for c in re.captures_iter(&s.original).flatten() {
            let m = c.get(1).unwrap();
            let name = m.as_str().to_string();
            let low = name.to_lowercase();
            if seen.contains(&low) || except.iter().any(|e| e.to_lowercase() == low) {
                continue;
            }
            seen.push(low.clone());
            if g.concepts
                .iter()
                .any(|c| c.terms.iter().any(|t| t.text.to_lowercase() == low))
            {
                continue;
            }
            // Modifiers must be common words (not names): "Clarity", not
            // "Pareto".
            let words: Vec<&str> = name.split_whitespace().collect();
            let common = words[..words.len() - 1].iter().all(|w| {
                w.split('-').all(|p| {
                    let l = p.to_lowercase();
                    crate::dict::tier(&l).is_some_and(|t| t <= 50)
                        && !crate::dict::words().contains_key(p)
                })
            });
            if !common {
                continue;
            }
            // Defined or cited anywhere in the document.
            let cited = a.sentences.iter().any(|o| {
                defines(&o.original, &name)
                    || o.original.contains(&format!("{name}]("))
                    || o.original.contains(&format!("[{name}]"))
            });
            if cited {
                continue;
            }
            let from = s.original[..m.start()].chars().count();
            let to = from + name.chars().count();
            out.push(Hit::at(a, si, from, to));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schwartz_hearst() {
        assert_eq!(
            long_form("IaaS", "we offer infrastructure as a service").as_deref(),
            Some("infrastructure as a service")
        );
        assert_eq!(
            long_form("HMM", "using a hidden Markov model").as_deref(),
            Some("hidden Markov model")
        );
        assert_eq!(long_form("XYZ", "nothing aligns here"), None);
        assert_eq!(acronym("APIs"), Some("API"));
        assert_eq!(acronym("IaaS"), Some("IaaS"));
        assert_eq!(acronym("GitHub"), None);
        assert_eq!(acronym("OpenAI"), None);
        assert_eq!(acronym("NOT"), None);
        assert_eq!(acronym("III"), None);
    }
}
