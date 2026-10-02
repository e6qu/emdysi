//! Document-structure checks over the block tree of a Markdown document:
//! heading hierarchy, empty and stacked sections, paragraph length, walls
//! of text, a conclusion held back to the end, and parallel form of sibling
//! headings and list items (from the grammar's analysis of each).
//!
//! The checks follow published style guides: markdownlint's heading rules
//! (MD001, MD025), Microsoft's guidance on headings (no two headings in a
//! row, at least two subsections, parallel structure), plainlanguage.gov's
//! paragraph limits (150 words, never more than 250) and the "bottom line
//! up front" rule of plainlanguage.gov, GOV.UK and Microsoft.

use std::collections::HashSet;
use std::ops::Range;

use emdysi_text::blocks::BlockKind;
use fancy_regex::Regex;

use crate::{Analysis, Format};

/// A finding of a document-level check.
#[derive(Debug, Clone)]
pub struct Hit {
    /// The sentence the finding is about, if any (for rule scopes).
    pub sentence: Option<usize>,
    /// Byte range in the source.
    pub range: Range<usize>,
    /// The text there (for `{match}`).
    pub text: String,
    /// Values for other message placeholders.
    pub vars: Vec<(&'static str, String)>,
    pub replacement: Option<String>,
    pub suggestions: Vec<String>,
}

impl Hit {
    pub fn new(sentence: Option<usize>, range: Range<usize>, text: impl Into<String>) -> Hit {
        Hit {
            sentence,
            range,
            text: text.into(),
            vars: Vec::new(),
            replacement: None,
            suggestions: Vec::new(),
        }
    }

    pub fn var(mut self, k: &'static str, v: impl ToString) -> Hit {
        self.vars.push((k, v.to_string()));
        self
    }

    /// A finding on characters `[from, to)` of sentence `s`.
    pub fn at(a: &Analysis, s: usize, from: usize, to: usize) -> Hit {
        let text: String = a.sentences[s]
            .original
            .chars()
            .skip(from)
            .take(to.saturating_sub(from))
            .collect();
        Hit::new(Some(s), a.source_range(s, from, to), text)
    }
}

#[derive(Debug, Clone)]
pub enum StructureCheck {
    /// A heading more than one level below the previous one (MD001).
    HeadingIncrement,
    /// More than one top-level heading (MD025).
    SingleH1,
    /// A heading with nothing under it before the next heading of the same
    /// or a higher level, or the end of the document.
    EmptySection,
    /// A heading followed directly by a subheading, with no text between.
    StackedHeadings,
    /// A section with exactly one subsection.
    LoneSubsection,
    /// Headings deeper than `max`.
    Depth { max: u8 },
    /// A run of paragraphs with no heading, list, table or code between,
    /// longer than `max_words` words or `max_paragraphs` paragraphs.
    WallOfText {
        max_words: usize,
        max_paragraphs: usize,
    },
    /// A paragraph longer than `max_words` words or `max_sentences`
    /// sentences.
    ParagraphLength {
        max_words: usize,
        max_sentences: usize,
    },
    /// The last second-level heading is a conclusion or summary: the main
    /// point is held back to the end.
    ConclusionAtEnd { pattern: Regex },
}

/// Grammatical form of a heading or list item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Form {
    Imperative,
    Gerund,
    Infinitive,
    Question,
    Statement,
    NounPhrase,
    /// "Label: description" (a definition list written as a list).
    Labelled,
}

impl Form {
    pub fn name(self) -> &'static str {
        match self {
            Form::Imperative => "an instruction (imperative)",
            Form::Gerund => "an -ing phrase",
            Form::Infinitive => "a to-infinitive",
            Form::Question => "a question",
            Form::Statement => "a full sentence",
            Form::NounPhrase => "a noun phrase",
            Form::Labelled => "a 'label: description' item",
        }
    }

    pub fn plural(self) -> &'static str {
        match self {
            Form::Imperative => "instructions",
            Form::Gerund => "-ing phrases",
            Form::Infinitive => "to-infinitives",
            Form::Question => "questions",
            Form::Statement => "full sentences",
            Form::NounPhrase => "noun phrases",
            Form::Labelled => "'label: description' items",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParallelOf {
    Headings,
    ListItems,
}

/// Source range of a whole block.
pub fn block_range(a: &Analysis, b: usize) -> Range<usize> {
    let bl = &a.blocks[b];
    bl.source_range(0..bl.text.len())
}

/// The first sentence of a block, if it has one.
pub fn block_sentence(a: &Analysis, b: usize) -> Option<usize> {
    a.sentences.iter().position(|s| s.block == b)
}

fn block_hit(a: &Analysis, b: usize) -> Hit {
    Hit::new(
        block_sentence(a, b),
        block_range(a, b),
        a.blocks[b].text.clone(),
    )
}

fn words(text: &str) -> usize {
    text.split_whitespace()
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .count()
}

/// Blocks that open the document or a top-level (H1 or H2) section: the
/// first paragraph after the start or after such a heading, if nothing
/// else (a subheading, list, quote or code) comes first.
pub fn lead_blocks(a: &Analysis) -> HashSet<usize> {
    let mut out = HashSet::new();
    let mut want = true;
    for (i, b) in a.blocks.iter().enumerate() {
        match b.kind {
            BlockKind::Heading(l) => want = l <= 2,
            BlockKind::Paragraph if want => {
                out.insert(i);
                want = false;
            }
            _ => want = false,
        }
    }
    out
}

pub fn run_structure(check: &StructureCheck, a: &Analysis) -> Vec<Hit> {
    let mut out = Vec::new();
    let headings: Vec<(usize, u8)> = a
        .blocks
        .iter()
        .enumerate()
        .filter_map(|(i, b)| match b.kind {
            BlockKind::Heading(l) => Some((i, l)),
            _ => None,
        })
        .collect();
    let markdown = a.format == Format::Markdown;
    match check {
        StructureCheck::HeadingIncrement => {
            for w in headings.windows(2) {
                let ((_, p), (b, l)) = (w[0], w[1]);
                if l > p + 1 {
                    out.push(block_hit(a, b).var("from", p).var("to", l));
                }
            }
        }
        StructureCheck::SingleH1 => {
            let h1: Vec<usize> = headings.iter().filter(|h| h.1 == 1).map(|h| h.0).collect();
            for &b in h1.iter().skip(1) {
                out.push(block_hit(a, b).var("count", h1.len()));
            }
        }
        StructureCheck::EmptySection => {
            for (k, &(b, l)) in headings.iter().enumerate() {
                let next = headings.get(k + 1);
                let body_end = next.map_or(a.blocks.len(), |n| n.0);
                let empty = body_end == b + 1 && next.is_none_or(|n| n.1 <= l);
                if empty {
                    out.push(block_hit(a, b).var("level", l));
                }
            }
        }
        StructureCheck::StackedHeadings => {
            for w in headings.windows(2) {
                let ((b, l), (nb, nl)) = (w[0], w[1]);
                if nb == b + 1 && nl > l {
                    out.push(block_hit(a, b).var("next", a.blocks[nb].text.clone()));
                }
            }
        }
        StructureCheck::LoneSubsection => {
            for (k, &(_, l)) in headings.iter().enumerate() {
                let children: Vec<usize> = headings[k + 1..]
                    .iter()
                    .take_while(|h| h.1 > l)
                    .filter(|h| h.1 == l + 1)
                    .map(|h| h.0)
                    .collect();
                if children.len() == 1 {
                    out.push(block_hit(a, children[0]));
                }
            }
        }
        StructureCheck::Depth { max } => {
            for &(b, l) in &headings {
                if l > *max {
                    out.push(block_hit(a, b).var("level", l).var("max", max));
                }
            }
        }
        StructureCheck::WallOfText {
            max_words,
            max_paragraphs,
        } if markdown => {
            let mut run: Vec<usize> = Vec::new();
            let flush = |run: &mut Vec<usize>, out: &mut Vec<Hit>| {
                let n: usize = run.iter().map(|&b| words(&a.blocks[b].text)).sum();
                if run.len() > *max_paragraphs || n > *max_words {
                    let (first, last) = (run[0], *run.last().unwrap());
                    let range = block_range(a, first).start..block_range(a, last).end;
                    let text = a.source[range.clone()].to_string();
                    out.push(
                        Hit::new(block_sentence(a, first), range, text)
                            .var("count", n)
                            .var("paragraphs", run.len()),
                    );
                }
                run.clear();
            };
            for (i, b) in a.blocks.iter().enumerate() {
                if matches!(b.kind, BlockKind::Paragraph | BlockKind::Quote) {
                    run.push(i);
                } else if !run.is_empty() {
                    flush(&mut run, &mut out);
                }
            }
            if !run.is_empty() {
                flush(&mut run, &mut out);
            }
        }
        StructureCheck::WallOfText { .. } => {}
        StructureCheck::ParagraphLength {
            max_words,
            max_sentences,
        } => {
            for (i, b) in a.blocks.iter().enumerate() {
                if b.kind != BlockKind::Paragraph {
                    continue;
                }
                let n = words(&b.text);
                let sents = a.sentences.iter().filter(|s| s.block == i).count();
                if n > *max_words || sents > *max_sentences {
                    out.push(block_hit(a, i).var("count", n).var("sentences", sents));
                }
            }
        }
        StructureCheck::ConclusionAtEnd { pattern } => {
            let h2: Vec<usize> = headings.iter().filter(|h| h.1 == 2).map(|h| h.0).collect();
            if let Some(&last) = h2.last() {
                if h2.len() >= 2
                    && pattern
                        .is_match(a.blocks[last].text.trim())
                        .unwrap_or(false)
                {
                    out.push(block_hit(a, last));
                }
            }
        }
    }
    out
}

/// The forms a heading or list item can be read as, and the one its best
/// analysis gives.
#[derive(Debug, Clone)]
pub struct Forms {
    pub best: Form,
    pub possible: Vec<Form>,
}

/// The form of one analysis.
fn reading_form(s: &crate::Sentence, r: &emdysi_parse::Reading) -> Option<Form> {
    let mut ws: Vec<&emdysi_parse::Word> = r.words.iter().collect();
    ws.sort_by_key(|w| w.from);
    let w0 = ws
        .iter()
        .find(|w| w.surface.chars().any(char::is_alphabetic))?;
    let props = r.mrs.as_ref().and_then(|m| {
        let i = m.index.as_ref()?;
        m.props.get(i).cloned()
    });
    let prop = |k: &str| {
        props
            .as_ref()
            .and_then(|p| p.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone()))
    };
    let first = w0.surface.to_lowercase();
    let tensed = matches!(prop("TENSE").as_deref(), Some("pres" | "past" | "fut"));
    if emdysi_parse::is_strict(&r.root) && tensed && prop("SF").as_deref() == Some("prop") {
        return Some(Form::Statement);
    }
    if w0.le_type.starts_with("v_") {
        if first.ends_with("ing") {
            return Some(Form::Gerund);
        }
        if prop("SF").as_deref() == Some("comm") || first == w0.lemma {
            return Some(Form::Imperative);
        }
    }
    let _ = s;
    Some(Form::NounPhrase)
}

/// The forms of a heading or list item: from every analysis of its first
/// sentence, and from the parts of speech its first word can have, so
/// that "Test corpora" counts as both an instruction and a noun phrase.
pub fn forms_of(erg: &emdysi_parse::Erg, a: &Analysis, si: usize) -> Option<Forms> {
    static LABEL: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    // "Label: text", or "Label:" before a nested list; the label may hold
    // a parenthetical.
    let label = LABEL.get_or_init(|| Regex::new(r"^(?:[^:.!?]|\.\S){1,80}:(?:\s|$)").unwrap());
    let s = &a.sentences[si];
    let text = s.original.trim();
    let only = |f: Form| {
        Some(Forms {
            best: f,
            possible: vec![f],
        })
    };
    if text.is_empty() {
        return None;
    }
    // An item that ends with a colon introduces a nested list: a label.
    if text.ends_with(':') {
        return only(Form::Labelled);
    }
    if label.is_match(text).unwrap_or(false)
        && text.split(':').next().is_some_and(|l| {
            let mut depth = 0i32;
            let bare: String = l
                .chars()
                .filter(|&c| {
                    depth += match c {
                        '(' => 1,
                        ')' => -1,
                        _ => 0,
                    };
                    depth == 0 && c != ')'
                })
                .collect();
            bare.split_whitespace().count() <= 5
        })
    {
        return only(Form::Labelled);
    }
    if text.ends_with('?') {
        return only(Form::Question);
    }
    // The first word, skipping numbering ("1.", "(a)") and inline code.
    let masked: Vec<char> = s.text.chars().collect();
    let first = s.tokens.iter().find(|t| {
        t.form.chars().any(char::is_alphabetic)
            && !(t.form.chars().count() == 1 && text.starts_with('(') && t.from <= 1)
    })?;
    let code = masked
        .get(first.from..first.to)
        .is_some_and(|c| c.iter().all(|&c| c == 'x'))
        && !first.form.chars().all(|c| c == 'x');
    let word = first.form.to_lowercase();
    if word == "to" {
        return only(Form::Infinitive);
    }
    let mut possible = Vec::new();
    let add = |f: Form, p: &mut Vec<Form>| {
        if !p.contains(&f) {
            p.push(f);
        }
    };
    let mut best = None;
    if let Some(p) = &s.parse {
        // The best analysis only: further down, a fragment reading of
        // almost anything turns up. Lexical ambiguity ("Test corpora") is
        // added from the first word's entries below.
        for r in p.readings.iter().take(1) {
            if let Some(f) = reading_form(s, r) {
                best.get_or_insert(f);
                add(f, &mut possible);
            }
        }
    }
    let types = erg.word_types(&word);
    // A noun reading of the first word counts unless a determiner follows
    // it ("Start the service" is not a noun phrase).
    let next = s
        .tokens
        .iter()
        .skip_while(|t| t.from < first.to)
        .find(|t| t.form.chars().any(char::is_alphabetic))
        .map(|t| t.form.to_lowercase());
    let determiner = next.as_deref().is_some_and(|n| {
        matches!(
            n,
            "the"
                | "a"
                | "an"
                | "your"
                | "my"
                | "our"
                | "their"
                | "this"
                | "that"
                | "these"
                | "those"
                | "each"
                | "every"
                | "all"
                | "some"
                | "any"
                | "it"
                | "them"
        )
    });
    if code || types.is_empty() || (!determiner && types.iter().any(|t| !t.starts_with("v_"))) {
        add(Form::NounPhrase, &mut possible);
    }
    if word.ends_with("ing") && word.chars().count() > 4 {
        add(Form::Gerund, &mut possible);
        add(Form::NounPhrase, &mut possible);
    } else if !code
        && erg
            .lexicon
            .entries_starting(&word)
            .iter()
            .any(|&e| erg.le_types[erg.lexicon.entries[e].inst].starts_with("v_"))
    {
        add(Form::Imperative, &mut possible);
    }
    let best = best.or_else(|| possible.first().copied())?;
    add(best, &mut possible);
    Some(Forms { best, possible })
}

/// Sibling groups: headings of one level under the same parent, or the
/// items of one list (the first paragraph of each).
fn groups(a: &Analysis, of: ParallelOf) -> Vec<Vec<usize>> {
    let mut out: Vec<Vec<usize>> = Vec::new();
    match of {
        ParallelOf::Headings => {
            // Open groups by level; a heading closes all deeper groups.
            let mut open: Vec<(u8, Vec<usize>)> = Vec::new();
            for (i, b) in a.blocks.iter().enumerate() {
                let BlockKind::Heading(l) = b.kind else {
                    continue;
                };
                while open.last().is_some_and(|g| g.0 > l) {
                    out.push(open.pop().unwrap().1);
                }
                match open.last_mut() {
                    Some(g) if g.0 == l => g.1.push(i),
                    _ => open.push((l, vec![i])),
                }
            }
            out.extend(open.into_iter().map(|g| g.1));
        }
        ParallelOf::ListItems => {
            let mut seen = HashSet::new();
            let mut by_list: Vec<(usize, Vec<usize>)> = Vec::new();
            for (i, b) in a.blocks.iter().enumerate() {
                if b.kind != BlockKind::ListItem || b.list == 0 || !seen.insert(b.item) {
                    continue;
                }
                match by_list.iter_mut().find(|g| g.0 == b.list) {
                    Some(g) => g.1.push(i),
                    None => by_list.push((b.list, vec![i])),
                }
            }
            out.extend(by_list.into_iter().map(|g| g.1));
        }
    }
    out
}

/// Members of a sibling group that cannot be read in the form most of the
/// group shares. An item counts for every form it can be read as; a group
/// is checked when one form fits at least `majority` of its items.
pub fn run_parallel(
    erg: &emdysi_parse::Erg,
    a: &Analysis,
    of: ParallelOf,
    min_items: usize,
    majority: f64,
) -> Vec<Hit> {
    let mut out = Vec::new();
    for g in groups(a, of) {
        if g.len() < min_items {
            continue;
        }
        let forms: Vec<(usize, Forms)> = g
            .iter()
            .filter_map(|&b| block_sentence(a, b))
            .filter_map(|s| forms_of(erg, a, s).map(|f| (s, f)))
            .collect();
        if forms.len() < min_items {
            continue;
        }
        // The form that fits the most items; ties go to the form that is
        // the best reading of more items.
        let all = [
            Form::Imperative,
            Form::Gerund,
            Form::Infinitive,
            Form::Question,
            Form::Statement,
            Form::NounPhrase,
            Form::Labelled,
        ];
        let fits = |f: Form| {
            forms
                .iter()
                .filter(|(_, x)| x.possible.contains(&f))
                .count()
        };
        let best_of = |f: Form| forms.iter().filter(|(_, x)| x.best == f).count();
        let Some(top) = all.iter().copied().max_by_key(|&f| (fits(f), best_of(f))) else {
            continue;
        };
        let n = fits(top);
        if (n as f64) < majority * forms.len() as f64 || n == forms.len() {
            continue;
        }
        for (s, f) in &forms {
            if !f.possible.contains(&top) {
                let len = a.sentences[*s].original.chars().count();
                out.push(
                    Hit::at(a, *s, 0, len)
                        .var("form", f.best.name())
                        .var("majority", top.plural()),
                );
            }
        }
    }
    out
}
