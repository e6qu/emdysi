//! Checks on the semantics (MRS) of the best analysis of each sentence.
//!
//! Each check looks for a configuration of the ERG's Minimal Recursion
//! Semantics (see the ERG's "semantic fingerprints"):
//!
//! - missing comparand: a comparative (`comp` predicate) that is the main
//!   predication of the sentence and whose standard of comparison (`ARG2`)
//!   is not expressed: *this approach is better* (but not *a larger
//!   kitchen* or *more desks*);
//! - agentless passive: a tensed passive verb (its event has an
//!   information-structure `topic` constraint) whose `ARG1`, the actor, is
//!   unexpressed: *the report was written* (but not *fast-paced*);
//! - stacked negation: two or more negators (`neg`, `_no_q`, `_never_a_1`)
//!   in one sentence: *we did not see nothing*;
//! - bare demonstrative: *this* or *that* used as a whole noun phrase
//!   (`generic_entity` quantified by a demonstrative) at the start of a
//!   sentence: *This shows that ...*;
//! - tense shift: a declarative sentence whose main verb is in a different
//!   tense (past or present) than most declarative sentences of its
//!   paragraph.
//! - vacuous: a sentence whose content predicates (nouns, verbs,
//!   adjectives and adverbs) are, all but at most one in five, abstractions
//!   that say nothing on their own (|The future belongs to those who
//!   embrace change.|), with no name or number;
//! - aphorism: a short present-tense statement whose subject and other
//!   nouns are bare singular nouns (|Clarity beats complexity.|,
//!   |Communication is key.|).

use std::collections::HashMap;

use emdysi_parse::mrs::{Ep, Mrs, var_type};

use crate::Analysis;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemCheck {
    MissingComparand,
    AgentlessPassive,
    StackedNegation,
    BareDemonstrative,
    TenseShift,
    Vacuous,
    Aphorism,
}

/// Abstractions that carry no content of their own in a sentence, by the
/// lemma of their predicate.
const VAGUE: &[&str] = &[
    "thing",
    "way",
    "world",
    "future",
    "change",
    "success",
    "value",
    "growth",
    "journey",
    "potential",
    "possibility",
    "opportunity",
    "challenge",
    "innovation",
    "impact",
    "importance",
    "role",
    "key",
    "crucial",
    "essential",
    "important",
    "vital",
    "meaningful",
    "powerful",
    "true",
    "real",
    "clear",
    "embrace",
    "unlock",
    "inspire",
    "empower",
    "transform",
    "redefine",
    "matter",
    "belong",
    "require",
    "create",
    "drive",
    "shape",
    "connection",
    "trust",
    "power",
    "adaptation",
    "constant",
    "landscape",
    "era",
    "realm",
    "paradigm",
    "shift",
    "approach",
    "aspect",
    "factor",
    "truly",
    "deeply",
    "ultimately",
    "really",
    "new",
    "great",
    "lasting",
    "difference",
    "people",
    "life",
    "today",
    "tomorrow",
    "progress",
    "strength",
    "commitment",
    "passion",
    "vision",
    "purpose",
    "mindset",
    "excellence",
    "heart",
    "core",
    "foundation",
    "balance",
    "stay",
    "remain",
    "begin",
    "start",
    "lead",
    "help",
    "make",
    "endless",
    "limitless",
    "everything",
    "something",
    "possible",
    "unparalleled",
    "innovate",
    "transformative",
    "pivotal",
    "vibrant",
    "tapestry",
    "testament",
    "harness",
    "foster",
    "navigate",
    "leverage",
    "unique",
    "ever-evolving",
];

/// The lemma and part of speech of a surface predicate (|_future_n_1| is
/// |future|, |n|), or `None` for a grammar predicate.
fn lemma_pos(pred: &str) -> Option<(&str, &str)> {
    let rest = pred.strip_prefix('_')?;
    let mut parts = rest.split('_');
    let lemma = parts.next()?;
    let pos = parts.next()?;
    Some((lemma, pos))
}

/// A finding: sentence index, character span in the sentence, the text
/// there, and a detail for the message.
#[derive(Debug, Clone)]
pub struct Finding {
    pub sentence: usize,
    pub from: usize,
    pub to: usize,
    pub text: String,
    pub detail: String,
}

/// Number of occurrences of each variable in the EPs (labels excluded).
fn uses(m: &Mrs) -> HashMap<&str, usize> {
    let mut n: HashMap<&str, usize> = HashMap::new();
    for ep in &m.eps {
        for (_, v) in &ep.args {
            *n.entry(v.as_str()).or_default() += 1;
        }
    }
    n
}

/// A variable that nothing else refers to: an unexpressed argument.
fn unbound(m: &Mrs, counts: &HashMap<&str, usize>, v: &str) -> bool {
    matches!(var_type(v), "i" | "u" | "x") && counts.get(v).copied().unwrap_or(0) <= 1 && {
        // An x variable is expressed when some EP has it as ARG0.
        !m.eps.iter().any(|e| e.arg0() == Some(v))
    }
}

/// Participles mostly used as adjectives after "be" ("the evidence is
/// mixed"), as in `plain-style.passive`.
const ADJECTIVAL_PARTICIPLES: &[&str] = &[
    "mixed",
    "interested",
    "tired",
    "excited",
    "worried",
    "pleased",
    "thrilled",
    "concerned",
    "involved",
    "based",
    "located",
    "married",
    "related",
    "satisfied",
    "surprised",
    "confused",
    "disappointed",
    "bored",
    "scared",
    "convinced",
    "done",
    "gone",
    "finished",
    "supposed",
    "used",
];

/// Quantity adjectives: "more desks", "fewer meetings", "less time".
fn is_quantity(pred: &str) -> bool {
    matches!(pred, "much-many_a" | "little-few_a")
        || ["_few_a", "_many_a", "_much_a", "_little_a"]
            .iter()
            .any(|p| pred.starts_with(p))
}

fn span(ep: &Ep) -> Option<(usize, usize)> {
    ep.lnk
}

fn text_of(original: &str, from: usize, to: usize) -> String {
    original
        .chars()
        .skip(from)
        .take(to.saturating_sub(from))
        .collect()
}

fn tense(m: &Mrs) -> Option<String> {
    let i = m.index.as_ref()?;
    let props = m.props.get(i)?;
    let get = |k: &str| props.iter().find(|(p, _)| p == k).map(|(_, v)| v.as_str());
    if get("SF") != Some("prop") {
        return None;
    }
    match get("TENSE")? {
        t @ ("past" | "pres") => Some(t.to_string()),
        _ => None,
    }
}

pub fn run(check: SemCheck, a: &Analysis) -> Vec<Finding> {
    let mut out = Vec::new();
    let mrs_of = |si: usize| {
        let s = &a.sentences[si];
        // Only strict analyses: fragments and robust readings say little.
        if !s.strict() {
            return None;
        }
        s.best().and_then(|r| r.mrs.as_ref())
    };
    if check == SemCheck::TenseShift {
        // By paragraph: the majority tense among declarative sentences.
        let mut by_block: HashMap<usize, Vec<(usize, String)>> = HashMap::new();
        for si in 0..a.sentences.len() {
            if let Some(t) = mrs_of(si).and_then(tense) {
                by_block
                    .entry(a.sentences[si].block)
                    .or_default()
                    .push((si, t));
            }
        }
        for sents in by_block.values() {
            if sents.len() < 3 {
                continue;
            }
            let past = sents.iter().filter(|(_, t)| t == "past").count();
            let pres = sents.len() - past;
            let (major, minor) = if past > pres {
                ("past", "pres")
            } else {
                ("pres", "past")
            };
            // Only a clear majority (at least two thirds) sets the tense.
            if past.max(pres) * 3 < sents.len() * 2 {
                continue;
            }
            for (si, t) in sents {
                if t != minor {
                    continue;
                }
                let m = mrs_of(*si).unwrap();
                let main = m
                    .eps
                    .iter()
                    .find(|e| m.index.as_deref() == e.arg0() && e.lnk.is_some());
                let (from, to) = main
                    .and_then(span)
                    .unwrap_or((0, a.sentences[*si].original.chars().count()));
                let name = |t: &str| if t == "past" { "past" } else { "present" };
                out.push(Finding {
                    sentence: *si,
                    from,
                    to,
                    text: text_of(&a.sentences[*si].original, from, to),
                    detail: format!(
                        "this sentence is in the {} tense, the rest of the paragraph mostly in the {}",
                        name(minor),
                        name(major)
                    ),
                });
            }
        }
        out.sort_by_key(|f| (f.sentence, f.from));
        return out;
    }
    for si in 0..a.sentences.len() {
        let Some(m) = mrs_of(si) else { continue };
        let original = &a.sentences[si].original;
        let counts = uses(m);
        let mut push = |from: usize, to: usize, detail: String| {
            out.push(Finding {
                sentence: si,
                from,
                to,
                text: text_of(original, from, to),
                detail,
            });
        };
        match check {
            SemCheck::MissingComparand => {
                for ep in &m.eps {
                    if !(ep.pred.ends_with("_comp") || ep.pred == "comp") {
                        continue;
                    }
                    let Some(arg2) = ep.arg("ARG2") else { continue };
                    // "more desks", "fewer errors": a quantity, not a
                    // comparison that needs a standard.
                    let of = ep.arg("ARG1");
                    // Only the main predication ("this approach is
                    // better"): an attributive comparative ("a larger
                    // kitchen") usually draws its standard from context.
                    if of.is_none() || of != m.index.as_deref() {
                        continue;
                    }
                    if m.eps
                        .iter()
                        .any(|q| is_quantity(&q.pred) && q.arg0().is_some() && q.arg0() == of)
                    {
                        continue;
                    }
                    if unbound(m, &counts, arg2) {
                        if let Some((f, t)) = span(ep) {
                            push(f, t, String::new());
                        }
                    }
                }
            }
            SemCheck::AgentlessPassive => {
                for (e, rel, _) in &m.icons {
                    if rel != "topic" {
                        continue;
                    }
                    let Some(verb) = m
                        .eps
                        .iter()
                        .find(|ep| ep.arg0() == Some(e.as_str()) && ep.pred.contains("_v_"))
                    else {
                        continue;
                    };
                    // A finite passive clause ("was written"), not an
                    // adjectival participle ("fast-paced", "underrated").
                    let tensed = m
                        .props
                        .get(e)
                        .and_then(|p| p.iter().find(|(k, _)| k == "TENSE"))
                        .is_some_and(|(_, t)| matches!(t.as_str(), "past" | "pres" | "fut"));
                    if !tensed {
                        continue;
                    }
                    let Some(actor) = verb.arg("ARG1") else {
                        continue;
                    };
                    if unbound(m, &counts, actor) {
                        if let Some((f, t)) = span(verb) {
                            let word = text_of(original, f, t).to_lowercase();
                            if !ADJECTIVAL_PARTICIPLES.contains(&word.as_str()) {
                                push(f, t, String::new());
                            }
                        }
                    }
                }
            }
            SemCheck::StackedNegation => {
                let negs: Vec<&Ep> = m
                    .eps
                    .iter()
                    .filter(|ep| matches!(ep.pred.as_str(), "neg" | "_no_q" | "_never_a_1"))
                    .collect();
                if negs.len() >= 2 {
                    let from = negs.iter().filter_map(|e| span(e)).map(|s| s.0).min();
                    let to = negs.iter().filter_map(|e| span(e)).map(|s| s.1).max();
                    if let (Some(f), Some(t)) = (from, to) {
                        push(f, t, format!("{} negations", negs.len()));
                    }
                }
            }
            SemCheck::BareDemonstrative => {
                for ep in &m.eps {
                    if ep.pred != "generic_entity" {
                        continue;
                    }
                    let Some(x) = ep.arg0() else { continue };
                    let demonstrative = m.eps.iter().any(|q| {
                        q.arg0() == Some(x)
                            && matches!(q.pred.as_str(), "_this_q_dem" | "_that_q_dem")
                    });
                    if let (true, Some((f, t))) = (demonstrative, span(ep)) {
                        if f == 0 {
                            push(f, t, String::new());
                        }
                    }
                }
            }
            SemCheck::Vacuous => {
                // Names, numbers and dates are content.
                if m.eps.iter().any(|e| {
                    matches!(
                        e.pred.as_str(),
                        "named"
                            | "card"
                            | "ord"
                            | "yofc"
                            | "dofm"
                            | "mofy"
                            | "dofw"
                            | "numbered_hour"
                    )
                }) {
                    continue;
                }
                let content: Vec<&str> = m
                    .eps
                    .iter()
                    .filter_map(|e| lemma_pos(&e.pred))
                    .filter(|(l, pos)| {
                        matches!(*pos, "n" | "v" | "a")
                            && !matches!(*l, "be" | "have" | "do" | "get" | "go" | "can" | "will")
                    })
                    .map(|(l, _)| l)
                    .collect();
                let vague = content
                    .iter()
                    .filter(|l| VAGUE.contains(&l.to_lowercase().as_str()))
                    .count();
                if content.len() >= 3 && vague * 5 >= content.len() * 4 {
                    let n = original.trim_end().chars().count();
                    push(0, n, format!("{vague} of {} content words", content.len()));
                }
            }
            SemCheck::Aphorism => {
                if original.split_whitespace().count() > 7 {
                    continue;
                }
                let Some(index) = m.index.as_deref() else {
                    continue;
                };
                let props = |v: &str| m.props.get(v).cloned().unwrap_or_default();
                let prop =
                    |v: &str, k: &str| props(v).into_iter().find(|(p, _)| p == k).map(|(_, x)| x);
                if prop(index, "SF").as_deref() != Some("prop")
                    || prop(index, "TENSE").as_deref() != Some("pres")
                    || prop(index, "PROG").as_deref() == Some("+")
                    || prop(index, "PERF").as_deref() == Some("+")
                {
                    continue;
                }
                if m.eps.iter().any(|e| {
                    matches!(
                        e.pred.as_str(),
                        "named" | "card" | "pron" | "generic_entity"
                    ) || e.pred.starts_with("_this_q")
                        || e.pred.starts_with("_that_q")
                }) {
                    continue;
                }
                let Some(main) = m.eps.iter().find(|e| e.arg0() == Some(index)) else {
                    continue;
                };
                let Some(subject) = main.arg("ARG1") else {
                    continue;
                };
                // A bare noun phrase (udef_q) of one singular noun.
                let bare = m
                    .eps
                    .iter()
                    .any(|q| q.pred == "udef_q" && q.arg0() == Some(subject));
                let noun = m.eps.iter().find(|e| {
                    e.arg0() == Some(subject) && lemma_pos(&e.pred).is_some_and(|(_, p)| p == "n")
                });
                let singular = prop(subject, "NUM").as_deref() == Some("sg");
                // Nothing else modifies the subject (|Clarity beats
                // complexity|, not |Free software matters|).
                let modified = m.eps.iter().any(|e| {
                    e.arg("ARG1") == Some(subject)
                        && e.arg0() != Some(index)
                        && e.label != main.label
                });
                // Every other noun phrase is as bare (|beats complexity|,
                // |starts with discomfort|), not a particular thing (|Rust
                // prevents data races|).
                let others_bare = m
                    .eps
                    .iter()
                    .filter(|e| lemma_pos(&e.pred).is_some_and(|(_, p)| p == "n"))
                    .filter_map(|e| e.arg0())
                    .all(|x| {
                        m.eps
                            .iter()
                            .any(|q| q.pred == "udef_q" && q.arg0() == Some(x))
                            && prop(x, "NUM").as_deref() == Some("sg")
                            && !m
                                .eps
                                .iter()
                                .any(|c| c.pred == "compound" && c.arg("ARG1") == Some(x))
                    });
                if let (true, Some(_), true, false, true) =
                    (bare, noun, singular, modified, others_bare)
                {
                    let n = original.trim_end().chars().count();
                    push(0, n, String::new());
                }
            }
            SemCheck::TenseShift => unreachable!(),
        }
    }
    out
}
