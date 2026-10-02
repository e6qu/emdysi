//! Rule packs: TOML files of rules (see `docs/rules.md`).
//!
//! ```toml
//! [pack]
//! name = "example"
//!
//! [[rule]]
//! id = "example.delve"
//! kind = "words"            # words | regex | construction | coordination
//!                           # | sentence-length | density | spelling | grammar
//!                           # | grammar-errors | semantics | consistency
//!                           # | structure | parallel | acronyms | glossary
//!                           # | variants | coined-words | concept-names
//!                           # | hyphen-chain | ly-hyphen | noun-stack
//!                           # | existence | substitution
//! scope = "body"            # optional: all | heading | body | paragraph
//!                           # | list-item | lead
//! words = ["delve", "tapestry"]
//! match = "lemma"           # or "surface"
//! severity = "warning"      # error | warning | suggestion
//! message = "'{match}' is a common tell of machine-written prose."
//! ```

use crate::dict::{Variety, variants};
use std::collections::HashMap;

use emdysi_parse::Erg;
use fancy_regex::Regex;

use crate::structure::{Hit, ParallelOf, StructureCheck};
use crate::terms::{AcronymCheck, Concept, Glossary, GlossaryCheck};
use crate::toml::{Table, Value};
use crate::{Analysis, Diagnostic, Severity};

#[derive(Debug)]
pub struct PackError(pub String);

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PackError {}

#[derive(Debug, Clone)]
pub struct Pack {
    pub name: String,
    pub description: String,
    pub rules: Vec<Rule>,
    /// Glossary concepts (`[[concept]]`), shared by all rules.
    pub concepts: Vec<Concept>,
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub id: String,
    pub severity: Severity,
    pub message: String,
    pub description: String,
    pub kind: Kind,
    /// Where in a document the rule applies.
    pub scope: Scope,
    /// Documents (Markdown) the rule must flag, and documents it must not;
    /// run as tests.
    pub examples: Vec<String>,
    pub acceptable: Vec<String>,
    /// Where the rule or its word list comes from, and under which license,
    /// for rules adapted from other tools or style guides.
    pub source: Option<String>,
    pub license: Option<String>,
}

/// The part of a document a rule looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    All,
    /// Headings only.
    Heading,
    /// Everything but headings.
    Body,
    /// Paragraphs (not headings, list items, quotes or table cells).
    Paragraph,
    ListItem,
    /// The first paragraph of the document and of each top-level (H1 or
    /// H2) section.
    Lead,
}

impl Scope {
    fn parse(s: &str) -> Option<Scope> {
        match s {
            "all" => Some(Scope::All),
            "heading" => Some(Scope::Heading),
            "body" => Some(Scope::Body),
            "paragraph" => Some(Scope::Paragraph),
            "list-item" => Some(Scope::ListItem),
            "lead" => Some(Scope::Lead),
            _ => None,
        }
    }
}

/// A glob over names: `*` matches any run of characters.
#[derive(Debug, Clone)]
pub struct Glob(String);

impl Glob {
    pub fn matches(&self, s: &str) -> bool {
        fn go(p: &[u8], s: &[u8]) -> bool {
            match p.first() {
                None => s.is_empty(),
                Some(b'*') => (0..=s.len()).any(|i| go(&p[1..], &s[i..])),
                Some(&c) => s.first() == Some(&c) && go(&p[1..], &s[1..]),
            }
        }
        go(self.0.as_bytes(), s.as_bytes())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchOn {
    Lemma,
    Surface,
}

#[derive(Debug, Clone)]
pub enum Kind {
    /// Words or phrases, matched on lemmas (from the parse) or surface
    /// forms. `replace` maps a listed item to an automatic fix.
    Words {
        items: Vec<Vec<String>>,
        on: MatchOn,
        replace: HashMap<String, String>,
    },
    /// A regular expression over the sentence text. Sentences that match
    /// `unless` (e.g. a citation) are skipped.
    Regex {
        re: Regex,
        replace: Option<String>,
        unless: Option<Regex>,
    },
    /// Derivation nodes whose rule, lexical entry or lexical type matches.
    Construction {
        rules: Vec<Glob>,
        entries: Vec<Glob>,
        types: Vec<Glob>,
        /// If set, one of these words must occur within `window` tokens
        /// before the match (e.g. a form of *be* for passives).
        preceded_by: Vec<String>,
        window: usize,
        /// Matches whose parent node is one of these rules (or reaches one
        /// through lexical rules) are skipped, e.g. a passive participle
        /// converted to an adjective.
        not_parent: Vec<Glob>,
        /// Matched text (case-insensitive) to skip.
        except: Vec<String>,
    },
    /// Coordinations with exactly / at least this many conjuncts.
    Coordination { min: usize, max: usize },
    /// Sentences longer than `max` words.
    SentenceLength { max: usize },
    /// More than `max` occurrences of any of `chars` or `words` per
    /// `per` words, over the whole document.
    Density {
        chars: Vec<char>,
        words: Vec<String>,
        per: f64,
        max: f64,
    },
    /// Words the grammar's lexicon does not know.
    Spelling {
        min_length: usize,
        ignore: Vec<String>,
    },
    /// Sentences without a strict (fully grammatical) analysis.
    Grammar,
    /// Specific grammatical errors, named by the grammar-error variant of
    /// the ERG (mal-rules and robust lexical entries).
    GrammarErrors,
    /// A check on the semantics (MRS) of the best analysis.
    Semantics(SemCheck),
    /// American and British spellings (and British -ise and -ize) mixed in
    /// one document. The variety used most (or `prefer`) wins; the others
    /// are fixed to it.
    Consistency {
        prefer: Option<Variety>,
        prefer_suffix: Option<Variety>,
    },
    /// Document structure: heading hierarchy, section and paragraph size.
    Structure(StructureCheck),
    /// Sibling headings or list items whose grammatical form differs from
    /// the majority (or from a fixed `form`).
    Parallel(crate::structure::Parallel),
    /// Acronyms and initialisms defined on first use. Word-list entries at
    /// least as common as `known_tier` (and `known`) need no definition.
    Acronyms {
        check: AcronymCheck,
        known_tier: u8,
        known: Vec<String>,
    },
    /// Terms of the glossary (`[[concept]]` tables).
    Glossary(GlossaryCheck),
    /// One spelling per term: hyphen, space and case variants.
    Variants { min_length: usize },
    /// Words coined from a known word and an affix.
    CoinedWords { ignore: Vec<String> },
    /// Capitalized concept names of common words with a framework-like
    /// head noun.
    ConceptNames {
        heads: Vec<String>,
        except: Vec<String>,
    },
    /// Hyphen chains of three or more parts.
    HyphenChain { except: Vec<String> },
    /// A hyphen after an -ly adverb.
    LyHyphen,
    /// Noun-noun compounds of `min` to `max` nouns.
    NounStack { min: usize, max: usize },
    /// Any of a list of patterns (Vale's `existence`): `tokens` are joined
    /// into one pattern, between word boundaries unless `nonword`.
    Existence { re: Regex, exceptions: Vec<String> },
    /// Patterns with preferred replacements (Vale's `substitution`): `swap`
    /// maps a pattern to its replacement; alternatives are separated by
    /// `|`. Replacements are suggestions, automatic fixes only with
    /// `fix = true` and a single alternative.
    Substitution {
        swaps: Vec<(Regex, Vec<String>)>,
        fix: bool,
    },
}

fn get_str(t: &Table, k: &str) -> Option<String> {
    t.get(k).and_then(Value::as_str).map(String::from)
}

fn get_list(t: &Table, k: &str) -> Vec<String> {
    t.get(k).and_then(Value::as_str_list).unwrap_or_default()
}

fn globs(t: &Table, k: &str) -> Vec<Glob> {
    get_list(t, k).into_iter().map(Glob).collect()
}

impl Pack {
    /// A glossary file: `[[concept]]` tables only, as a pack without rules.
    pub fn parse_glossary(src: &str) -> Result<Pack, PackError> {
        let t = crate::toml::parse(src).map_err(|e| PackError(e.to_string()))?;
        if t.contains_key("rule") {
            return Err(PackError(
                "a glossary has no [[rule]] tables; load it with --pack".into(),
            ));
        }
        Ok(Pack {
            name: "glossary".into(),
            description: String::new(),
            rules: Vec::new(),
            concepts: crate::terms::parse_concepts(&t).map_err(PackError)?,
        })
    }

    pub fn parse(src: &str) -> Result<Pack, PackError> {
        let t = crate::toml::parse(src).map_err(|e| PackError(e.to_string()))?;
        let meta = t
            .get("pack")
            .and_then(Value::as_table)
            .cloned()
            .unwrap_or_default();
        let name = get_str(&meta, "name").ok_or_else(|| PackError("missing [pack] name".into()))?;
        let mut rules = Vec::new();
        for (i, r) in t
            .get("rule")
            .and_then(Value::as_array)
            .unwrap_or(&[])
            .iter()
            .enumerate()
        {
            let r = r
                .as_table()
                .ok_or_else(|| PackError(format!("rule {i} is not a table")))?;
            rules.push(
                Rule::from_table(r).map_err(|e| PackError(format!("rule {}: {}", i + 1, e.0)))?,
            );
        }
        let concepts = crate::terms::parse_concepts(&t).map_err(PackError)?;
        Ok(Pack {
            name,
            description: get_str(&meta, "description").unwrap_or_default(),
            rules,
            concepts,
        })
    }
}

impl Rule {
    fn from_table(t: &Table) -> Result<Rule, PackError> {
        let id = get_str(t, "id").ok_or_else(|| PackError("missing id".into()))?;
        let kind_name =
            get_str(t, "kind").ok_or_else(|| PackError(format!("{id}: missing kind")))?;
        let err = |m: &str| PackError(format!("{id}: {m}"));
        let num = |k: &str| t.get(k).and_then(Value::as_f64);
        let kind = match kind_name.as_str() {
            "words" => {
                let items: Vec<Vec<String>> = get_list(t, "words")
                    .iter()
                    .map(|w| {
                        w.to_lowercase()
                            .split_whitespace()
                            .map(String::from)
                            .collect()
                    })
                    .filter(|v: &Vec<String>| !v.is_empty())
                    .collect();
                if items.is_empty() {
                    return Err(err("words rule needs `words`"));
                }
                let on = match get_str(t, "match").as_deref() {
                    None | Some("lemma") => MatchOn::Lemma,
                    Some("surface") => MatchOn::Surface,
                    Some(o) => return Err(err(&format!("unknown match {o:?}"))),
                };
                let replace = t
                    .get("replace")
                    .and_then(Value::as_table)
                    .map(|m| {
                        m.iter()
                            .filter_map(|(k, v)| {
                                v.as_str().map(|s| (k.to_lowercase(), s.to_string()))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Kind::Words { items, on, replace }
            }
            "regex" => {
                let pat = get_str(t, "pattern").ok_or_else(|| err("regex rule needs `pattern`"))?;
                let re = Regex::new(&pat).map_err(|e| err(&format!("bad pattern: {e}")))?;
                let unless = get_str(t, "unless")
                    .map(|u| Regex::new(&u).map_err(|e| err(&format!("bad `unless` pattern: {e}"))))
                    .transpose()?;
                Kind::Regex {
                    re,
                    replace: get_str(t, "replace"),
                    unless,
                }
            }
            "construction" => {
                let k = Kind::Construction {
                    rules: globs(t, "rules"),
                    entries: globs(t, "entries"),
                    types: globs(t, "types"),
                    preceded_by: get_list(t, "preceded_by")
                        .iter()
                        .map(|w| w.to_lowercase())
                        .collect(),
                    window: num("window").unwrap_or(3.0) as usize,
                    not_parent: globs(t, "not_parent"),
                    except: get_list(t, "except")
                        .iter()
                        .map(|w| w.to_lowercase())
                        .collect(),
                };
                if let Kind::Construction {
                    rules,
                    entries,
                    types,
                    ..
                } = &k
                {
                    if rules.is_empty() && entries.is_empty() && types.is_empty() {
                        return Err(err("construction rule needs `rules`, `entries` or `types`"));
                    }
                }
                k
            }
            "coordination" => Kind::Coordination {
                min: num("min").unwrap_or(3.0) as usize,
                max: num("max").unwrap_or(f64::INFINITY).min(1e6) as usize,
            },
            "sentence-length" => Kind::SentenceLength {
                max: num("max").ok_or_else(|| err("sentence-length rule needs `max`"))? as usize,
            },
            "density" => Kind::Density {
                chars: get_list(t, "chars")
                    .iter()
                    .flat_map(|s| s.chars())
                    .collect(),
                words: get_list(t, "words")
                    .iter()
                    .map(|w| w.to_lowercase())
                    .collect(),
                per: num("per").unwrap_or(100.0),
                max: num("max").ok_or_else(|| err("density rule needs `max`"))?,
            },
            "spelling" => Kind::Spelling {
                min_length: num("min_length").unwrap_or(3.0) as usize,
                ignore: get_list(t, "ignore")
                    .iter()
                    .map(|w| w.to_lowercase())
                    .collect(),
            },
            "grammar" => Kind::Grammar,
            "grammar-errors" => Kind::GrammarErrors,
            "semantics" => {
                let c = get_str(t, "check").ok_or_else(|| err("semantics rule needs `check`"))?;
                Kind::Semantics(match c.as_str() {
                    "missing-comparand" => SemCheck::MissingComparand,
                    "agentless-passive" => SemCheck::AgentlessPassive,
                    "stacked-negation" => SemCheck::StackedNegation,
                    "bare-demonstrative" => SemCheck::BareDemonstrative,
                    "tense-shift" => SemCheck::TenseShift,
                    other => return Err(err(&format!("unknown semantics check {other:?}"))),
                })
            }
            "consistency" => {
                let variety = |k: &str, allowed: [&str; 2]| -> Result<Option<Variety>, PackError> {
                    match get_str(t, k) {
                        None => Ok(None),
                        Some(v) if allowed.contains(&v.as_str()) => Ok(Variety::parse(&v)),
                        Some(v) => Err(err(&format!(
                            "`{k}` must be {:?} or {:?}, not {v:?}",
                            allowed[0], allowed[1]
                        ))),
                    }
                };
                Kind::Consistency {
                    prefer: variety("prefer", ["us", "gb"])?,
                    prefer_suffix: variety("prefer_suffix", ["ise", "ize"])?,
                }
            }
            "structure" => {
                let c = get_str(t, "check").ok_or_else(|| err("structure rule needs `check`"))?;
                let n = |k: &str, d: f64| num(k).unwrap_or(d) as usize;
                Kind::Structure(match c.as_str() {
                    "heading-increment" => StructureCheck::HeadingIncrement,
                    "single-h1" => StructureCheck::SingleH1,
                    "empty-section" => StructureCheck::EmptySection,
                    "stacked-headings" => StructureCheck::StackedHeadings,
                    "lone-subsection" => StructureCheck::LoneSubsection,
                    "depth" => StructureCheck::Depth {
                        max: n("max", 4.0) as u8,
                    },
                    "wall-of-text" => StructureCheck::WallOfText {
                        max_words: n("max_words", 450.0),
                        max_paragraphs: n("max_paragraphs", 5.0),
                    },
                    "paragraph-length" => StructureCheck::ParagraphLength {
                        max_words: n("max_words", 150.0),
                        max_sentences: n("max_sentences", usize::MAX as f64),
                    },
                    "conclusion-at-end" => {
                        let pat = get_str(t, "pattern").unwrap_or_else(|| {
                            r"(?i)^(?:in )?(?:conclusions?|summary|key takeaways|takeaways|final thoughts|closing thoughts|wrapping up|the bottom line|bottom line|tl;dr)\W*$".to_string()
                        });
                        StructureCheck::ConclusionAtEnd {
                            pattern: Regex::new(&pat)
                                .map_err(|e| err(&format!("bad pattern: {e}")))?,
                        }
                    }
                    "vague-lead" => {
                        let pat = get_str(t, "pattern").unwrap_or_else(|| {
                            r"(?i)\b(?:today|nowadays|ever(?:-changing|-evolving)?|increasingly|landscape|world|era|age|journey|realm|more than ever|in recent years|rapidly|fast-paced|digital|businesses|organizations|organisations|companies|individuals|everyone|people)\b".to_string()
                        });
                        StructureCheck::VagueLead {
                            pattern: Regex::new(&pat)
                                .map_err(|e| err(&format!("bad pattern: {e}")))?,
                        }
                    }
                    "unnumbered-steps" => {
                        let pat = get_str(t, "pattern").unwrap_or_else(|| {
                            r"(?i)\b(?:then|next|first|second|third|finally|afterwards?|after that|once|when (?:done|finished)|steps?|in order)\b".to_string()
                        });
                        StructureCheck::UnnumberedSteps {
                            pattern: Regex::new(&pat)
                                .map_err(|e| err(&format!("bad pattern: {e}")))?,
                        }
                    }
                    o => return Err(err(&format!("unknown structure check {o:?}"))),
                })
            }
            "parallel" => Kind::Parallel(crate::structure::Parallel {
                of: match get_str(t, "of").as_deref() {
                    Some("headings") => ParallelOf::Headings,
                    Some("list-items") => ParallelOf::ListItems,
                    _ => {
                        return Err(err(
                            "parallel rule needs `of` = \"headings\" or \"list-items\"",
                        ));
                    }
                },
                min_items: num("min_items").unwrap_or(3.0) as usize,
                majority: num("majority").unwrap_or(0.75),
                ordered: t.get("ordered").and_then(Value::as_bool).unwrap_or(false),
                form: match get_str(t, "form") {
                    None => None,
                    Some(f) => Some(
                        crate::structure::Form::parse(&f)
                            .ok_or_else(|| err(&format!("unknown form {f:?}")))?,
                    ),
                },
            }),
            "acronyms" => {
                let c = get_str(t, "check").ok_or_else(|| err("acronyms rule needs `check`"))?;
                Kind::Acronyms {
                    check: match c.as_str() {
                        "undefined" => AcronymCheck::Undefined,
                        "defined-after-use" => AcronymCheck::DefinedAfterUse,
                        "used-once" => AcronymCheck::UsedOnce,
                        "redefined" => AcronymCheck::Redefined,
                        "first-use-in-heading" => AcronymCheck::FirstUseInHeading,
                        o => return Err(err(&format!("unknown acronyms check {o:?}"))),
                    },
                    known_tier: num("known_tier").unwrap_or(50.0) as u8,
                    known: get_list(t, "known"),
                }
            }
            "glossary" => {
                let c = get_str(t, "check").ok_or_else(|| err("glossary rule needs `check`"))?;
                Kind::Glossary(match c.as_str() {
                    "deprecated" => GlossaryCheck::Deprecated,
                    "casing" => GlossaryCheck::Casing,
                    o => return Err(err(&format!("unknown glossary check {o:?}"))),
                })
            }
            "variants" => Kind::Variants {
                min_length: num("min_length").unwrap_or(5.0) as usize,
            },
            "coined-words" => Kind::CoinedWords {
                ignore: get_list(t, "ignore")
                    .iter()
                    .map(|w| w.to_lowercase())
                    .collect(),
            },
            "concept-names" => Kind::ConceptNames {
                heads: get_list(t, "heads"),
                except: get_list(t, "except"),
            },
            "hyphen-chain" => Kind::HyphenChain {
                except: get_list(t, "except")
                    .iter()
                    .map(|w| w.to_lowercase())
                    .collect(),
            },
            "ly-hyphen" => Kind::LyHyphen,
            "existence" | "substitution" => {
                let ignorecase = t
                    .get("ignorecase")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let nonword = t.get("nonword").and_then(Value::as_bool).unwrap_or(false);
                let wrap = |p: &str| {
                    let p = if nonword {
                        format!("(?:{p})")
                    } else {
                        format!(r"(?<![\w-])(?:{p})(?![\w-])")
                    };
                    if ignorecase { format!("(?i){p}") } else { p }
                };
                if kind_name == "existence" {
                    let tokens = get_list(t, "tokens");
                    if tokens.is_empty() {
                        return Err(err("existence rule needs `tokens`"));
                    }
                    let pat = wrap(&tokens.join("|"));
                    Kind::Existence {
                        re: Regex::new(&pat).map_err(|e| err(&format!("bad tokens: {e}")))?,
                        exceptions: get_list(t, "exceptions"),
                    }
                } else {
                    let table = t
                        .get("swap")
                        .and_then(Value::as_table)
                        .ok_or_else(|| err("substitution rule needs `swap`"))?;
                    let mut swaps = Vec::new();
                    for (k, v) in table {
                        let rep = v
                            .as_str()
                            .ok_or_else(|| err("`swap` values must be strings"))?;
                        let re = Regex::new(&wrap(k))
                            .map_err(|e| err(&format!("bad swap pattern {k:?}: {e}")))?;
                        swaps.push((re, rep.split('|').map(|r| r.trim().to_string()).collect()));
                    }
                    Kind::Substitution {
                        swaps,
                        fix: t.get("fix").and_then(Value::as_bool).unwrap_or(false),
                    }
                }
            }
            "noun-stack" => Kind::NounStack {
                min: num("min").unwrap_or(3.0) as usize,
                max: num("max").unwrap_or(f64::INFINITY).min(1e6) as usize,
            },
            k => return Err(err(&format!("unknown kind {k:?}"))),
        };
        let scope = match get_str(t, "scope") {
            None => Scope::All,
            Some(s) => Scope::parse(&s).ok_or_else(|| err(&format!("unknown scope {s:?}")))?,
        };
        let severity = match get_str(t, "severity") {
            None => Severity::Warning,
            Some(s) => {
                Severity::parse(&s).ok_or_else(|| err(&format!("unknown severity {s:?}")))?
            }
        };
        Ok(Rule {
            message: get_str(t, "message").unwrap_or_else(|| id.clone()),
            description: get_str(t, "description").unwrap_or_default(),
            id,
            severity,
            kind,
            scope,
            examples: get_list(t, "examples"),
            acceptable: get_list(t, "acceptable"),
            source: get_str(t, "source"),
            license: get_str(t, "license"),
        })
    }

    /// Whether sentence `s` is in the rule's scope.
    fn in_scope(&self, a: &Analysis, s: usize, lead: &std::collections::HashSet<usize>) -> bool {
        use emdysi_text::blocks::BlockKind as B;
        let b = a.sentences[s].block;
        let kind = a.blocks[b].kind;
        match self.scope {
            Scope::All => true,
            Scope::Heading => matches!(kind, B::Heading(_)),
            Scope::Body => !matches!(kind, B::Heading(_)),
            Scope::Paragraph => kind == B::Paragraph,
            Scope::ListItem => kind == B::ListItem,
            Scope::Lead => lead.contains(&b),
        }
    }

    fn hit_diag(&self, h: Hit) -> Diagnostic {
        let mut message = self.message.replace("{match}", &h.text);
        for (k, v) in &h.vars {
            message = message.replace(&format!("{{{k}}}"), v);
        }
        Diagnostic {
            rule: self.id.clone(),
            severity: self.severity,
            message,
            range: h.range,
            replacement: h.replacement,
            suggestions: h.suggestions,
            sentence: h.sentence,
        }
    }

    fn diag(&self, a: &Analysis, s: usize, from: usize, to: usize, matched: &str) -> Diagnostic {
        Diagnostic {
            rule: self.id.clone(),
            severity: self.severity,
            message: self.message.replace("{match}", matched),
            range: a.source_range(s, from, to),
            replacement: None,
            suggestions: Vec::new(),
            sentence: Some(s),
        }
    }

    /// Run the rule over an analysis, with the glossary of all loaded packs.
    pub fn run(&self, erg: &Erg, a: &Analysis, g: &Glossary, out: &mut Vec<Diagnostic>) {
        let mut found = Vec::new();
        self.run_all(erg, a, g, &mut found);
        if self.scope != Scope::All {
            let lead = crate::structure::lead_blocks(a);
            found.retain(|d| d.sentence.is_none_or(|s| self.in_scope(a, s, &lead)));
        }
        out.extend(found);
    }

    fn run_all(&self, erg: &Erg, a: &Analysis, g: &Glossary, out: &mut Vec<Diagnostic>) {
        let hits = match &self.kind {
            Kind::Structure(c) => Some(crate::structure::run_structure(c, erg, a)),
            Kind::Parallel(p) => Some(crate::structure::run_parallel(erg, a, p)),
            Kind::Acronyms {
                check,
                known_tier,
                known,
            } => Some(crate::terms::run_acronyms(*check, a, *known_tier, known, g)),
            Kind::Glossary(c) => Some(crate::terms::run_glossary(*c, g, a)),
            Kind::Variants { min_length } => Some(crate::terms::run_variants(a, *min_length, g)),
            Kind::CoinedWords { ignore } => Some(crate::terms::run_coined_words(erg, a, g, ignore)),
            Kind::ConceptNames { heads, except } => {
                Some(crate::terms::run_concept_names(a, heads, g, except))
            }
            Kind::HyphenChain { except } => Some(crate::compounds::run_kebab(a, except)),
            Kind::LyHyphen => Some(crate::compounds::run_ly_hyphen(a)),
            Kind::NounStack { min, max } => {
                Some(crate::compounds::run_noun_stacks(a, *min, *max, g))
            }
            _ => None,
        };
        if let Some(hits) = hits {
            out.extend(hits.into_iter().map(|h| self.hit_diag(h)));
            return;
        }
        match &self.kind {
            Kind::Words { items, on, replace } => self.run_words(a, items, *on, replace, out),
            Kind::Existence { re, exceptions } => {
                for (si, s) in a.sentences.iter().enumerate() {
                    // Match on the text with inline code masked.
                    for m in re.find_iter(&s.text).flatten() {
                        let text = &s.original[m.start()..m.end()];
                        if exceptions.iter().any(|e| e.eq_ignore_ascii_case(text)) {
                            continue;
                        }
                        let from = s.text[..m.start()].chars().count();
                        let to = from + m.as_str().chars().count();
                        out.push(self.diag(a, si, from, to, text));
                    }
                }
            }
            Kind::Substitution { swaps, fix } => {
                for (si, s) in a.sentences.iter().enumerate() {
                    let mut taken: Vec<(usize, usize)> = Vec::new();
                    for (re, reps) in swaps {
                        for m in re.find_iter(&s.text).flatten() {
                            if taken.iter().any(|&(f, t)| m.start() < t && f < m.end()) {
                                continue;
                            }
                            let text = &s.original[m.start()..m.end()];
                            // The pattern may match its own replacement.
                            if reps.iter().any(|r| r == text) {
                                continue;
                            }
                            taken.push((m.start(), m.end()));
                            let from = s.text[..m.start()].chars().count();
                            let to = from + m.as_str().chars().count();
                            let reps: Vec<String> =
                                reps.iter().map(|r| match_case(text, r)).collect();
                            let mut d = self.diag(a, si, from, to, text);
                            d.message = d.message.replace("{replacement}", &reps.join("' or '"));
                            if *fix && reps.len() == 1 {
                                d.replacement = Some(reps[0].clone());
                            } else {
                                d.suggestions = reps;
                            }
                            out.push(d);
                        }
                    }
                }
            }
            Kind::Regex {
                re,
                replace,
                unless,
            } => {
                for (si, s) in a.sentences.iter().enumerate() {
                    if unless
                        .as_ref()
                        .is_some_and(|u| u.is_match(&s.original).unwrap_or(false))
                    {
                        continue;
                    }
                    for m in re.captures_iter(s.original.as_str()).flatten() {
                        let m0 = m.get(0).unwrap();
                        let from = s.original[..m0.start()].chars().count();
                        let to = from + m0.as_str().chars().count();
                        let mut d = self.diag(a, si, from, to, m0.as_str());
                        if let Some(rep) = replace {
                            let mut expanded = String::new();
                            m.expand(rep, &mut expanded);
                            d.replacement = Some(expanded);
                        }
                        out.push(d);
                    }
                }
            }
            Kind::Construction {
                rules,
                entries,
                types,
                preceded_by,
                window,
                not_parent,
                except,
            } => {
                for (si, s) in a.sentences.iter().enumerate() {
                    let Some(r) = s.best() else { continue };
                    for n in &r.nodes {
                        // The parent, or an ancestor through a chain of
                        // lexical rules (e.g. "under-" then conversion).
                        let mut p = n.parent;
                        let mut skip = false;
                        while let Some(pi) = p {
                            let name = &r.nodes[pi].name;
                            if not_parent.iter().any(|g| g.matches(name)) {
                                skip = true;
                                break;
                            }
                            if !name.ends_with("lr") {
                                break;
                            }
                            p = r.nodes[pi].parent;
                        }
                        if skip {
                            continue;
                        }
                        let hit = if n.leaf {
                            entries.iter().any(|g| g.matches(&n.name))
                                || (!types.is_empty()
                                    && r.words.iter().any(|w| {
                                        w.entry == n.name
                                            && w.from == n.from
                                            && types.iter().any(|g| g.matches(&w.le_type))
                                    }))
                        } else {
                            rules.iter().any(|g| g.matches(&n.name))
                        };
                        let context_ok = preceded_by.is_empty() || {
                            let before: Vec<&emdysi_parse::InputToken> =
                                s.tokens.iter().filter(|t| t.to <= n.from).collect();
                            before
                                .iter()
                                .rev()
                                .take(*window)
                                .any(|t| preceded_by.contains(&t.form.to_lowercase()))
                        };
                        if hit && context_ok {
                            let text: String = s
                                .original
                                .chars()
                                .skip(n.from)
                                .take(n.to - n.from)
                                .collect();
                            if except.contains(&text.to_lowercase()) {
                                continue;
                            }
                            out.push(self.diag(a, si, n.from, n.to, &text));
                        }
                    }
                }
            }
            Kind::Coordination { min, max } => {
                for (si, s) in a.sentences.iter().enumerate() {
                    let Some(r) = s.best() else { continue };
                    for (ni, n) in r.nodes.iter().enumerate() {
                        let is_top = |name: &str| name.contains("_crd") && name.ends_with("-t_c");
                        if n.leaf || !is_top(&n.name) {
                            continue;
                        }
                        // Conjuncts: the top rule joins two; each mid rule
                        // on the way down adds one.
                        let mut count = 2;
                        let mut cur = ni;
                        loop {
                            let next = r.nodes[cur].children.iter().copied().find(|&c| {
                                r.nodes[c].name.contains("_crd")
                                    && (r.nodes[c].name.ends_with("-m_c")
                                        || r.nodes[c].name.ends_with("-im_c"))
                            });
                            match next {
                                Some(c) => {
                                    count += 1;
                                    cur = c;
                                }
                                None => break,
                            }
                        }
                        if count >= *min && count <= *max {
                            let text: String = s
                                .original
                                .chars()
                                .skip(n.from)
                                .take(n.to - n.from)
                                .collect();
                            out.push(self.diag(a, si, n.from, n.to, &text));
                        }
                    }
                }
            }
            Kind::SentenceLength { max } => {
                for (si, s) in a.sentences.iter().enumerate() {
                    let n = s.word_count();
                    if n > *max {
                        let len = s.original.chars().count();
                        let mut d = self.diag(a, si, 0, len, &s.original);
                        d.message = d
                            .message
                            .replace("{count}", &n.to_string())
                            .replace("{max}", &max.to_string());
                        out.push(d);
                    }
                }
            }
            Kind::Density {
                chars,
                words,
                per,
                max,
            } => {
                let mut total_words = 0usize;
                let mut hits: Vec<(usize, usize, usize, String)> = Vec::new();
                for (si, s) in a.sentences.iter().enumerate() {
                    total_words += s.word_count();
                    for (ci, c) in s.original.chars().enumerate() {
                        if chars.contains(&c) {
                            hits.push((si, ci, ci + 1, c.to_string()));
                        }
                    }
                    for t in &s.tokens {
                        if words.contains(&t.form.to_lowercase()) {
                            hits.push((si, t.from, t.to, t.form.clone()));
                        }
                    }
                }
                let rate = hits.len() as f64 * per / total_words.max(1) as f64;
                if rate > *max {
                    for (si, from, to, m) in hits {
                        let mut d = self.diag(a, si, from, to, &m);
                        d.message = d
                            .message
                            .replace("{count}", &format!("{rate:.1}"))
                            .replace("{max}", &format!("{max}"))
                            .replace("{per}", &format!("{per}"));
                        out.push(d);
                    }
                }
            }
            Kind::Spelling { min_length, ignore } => {
                for (si, s) in a.sentences.iter().enumerate() {
                    for t in &s.tokens {
                        let w = &t.form;
                        // Only plain lower-case words: names, acronyms, code
                        // and numbers are out of scope.
                        if w.chars().count() < *min_length
                            || !w.chars().all(|c| c.is_lowercase() || c == '\'' || c == '-')
                            || w.contains("--")
                            || s.text
                                .chars()
                                .skip(t.from)
                                .take(t.to - t.from)
                                .all(|c| c == 'x')
                            || ignore.contains(w)
                            || g.knows_word(w)
                        {
                            continue;
                        }
                        let known = |p: &str| crate::dict::tier(p).is_some() || erg.known_word(p);
                        let parts: Vec<&str> = w
                            .split(['-', '\'', '’'])
                            .filter(|p| !p.is_empty())
                            .collect();
                        if known(w) || parts.iter().all(|p| known(p)) {
                            continue;
                        }
                        // A known word plus an affix ("promptability") is a
                        // coinage, not a misspelling: see `coined-words`.
                        if crate::terms::novel_derivation(erg, w).is_some()
                            && suggestions(erg, w, 1).0.is_empty()
                        {
                            continue;
                        }
                        let (sugg, confident) = suggestions(erg, w, 5);
                        let mut d = self.diag(a, si, t.from, t.to, w);
                        d.message = d.message.replace(
                            "{suggestion}",
                            &sugg
                                .first()
                                .cloned()
                                .map(|x| format!(" Did you mean '{x}'?"))
                                .unwrap_or_default(),
                        );
                        // Fix automatically only when one candidate is clearly
                        // the most plausible typo, or the only one.
                        let single = sugg.len() == 1
                            && edit_distance(w, &sugg[0]) == 1
                            && w.chars().count() >= 4;
                        if confident || single {
                            d.replacement = sugg.first().cloned();
                        }
                        d.suggestions = sugg;
                        out.push(d);
                    }
                }
            }
            Kind::Consistency {
                prefer,
                prefer_suffix,
            } => self.run_consistency(a, *prefer, *prefer_suffix, out),
            Kind::Semantics(check) => {
                for f in crate::semantics::run(*check, a) {
                    let mut d = self.diag(a, f.sentence, f.from, f.to, &f.text);
                    d.message = d.message.replace("{detail}", &f.detail);
                    out.push(d);
                }
            }
            Kind::GrammarErrors => {
                for (si, s) in a.sentences.iter().enumerate() {
                    for e in grammar_errors(s) {
                        let mut d = self.diag(a, si, e.from, e.to, &e.text);
                        d.message = d
                            .message
                            .replace("{feedback}", &e.feedback)
                            .replace("{code}", &e.code);
                        out.push(d);
                    }
                }
            }
            Kind::Grammar => {
                for (si, s) in a.sentences.iter().enumerate() {
                    let Some(p) = &s.parse else { continue };
                    // A named error is reported by `grammar-errors` instead.
                    if !grammar_errors(s).is_empty() {
                        continue;
                    }
                    let block_kind = a.blocks[s.block].kind;
                    if matches!(
                        block_kind,
                        emdysi_text::blocks::BlockKind::Heading(_)
                            | emdysi_text::blocks::BlockKind::TableCell
                    ) {
                        continue;
                    }
                    if p.readings.is_empty() || !s.strict() {
                        let len = s.original.chars().count();
                        let mut d = self.diag(a, si, 0, len, &s.original);
                        let none = p.readings.iter().all(|r| r.root == "fragment");
                        let why = if none {
                            if p.exhausted {
                                "too complex to analyse in time"
                            } else {
                                "no grammatical analysis found"
                            }
                        } else {
                            "only a fragment or informal analysis found"
                        };
                        d.message = d.message.replace("{reason}", why);
                        out.push(d);
                    }
                }
            }
            _ => {} // document-level kinds, run above
        }
    }

    fn run_words(
        &self,
        a: &Analysis,
        items: &[Vec<String>],
        on: MatchOn,
        replace: &HashMap<String, String>,
        out: &mut Vec<Diagnostic>,
    ) {
        for (si, s) in a.sentences.iter().enumerate() {
            // Units: (lemma or surface, from, to), from the best reading when
            // there is one, else from the tokens.
            let units: Vec<(String, usize, usize)> = match (on, s.best()) {
                (MatchOn::Lemma, Some(r)) => r
                    .words
                    .iter()
                    .flat_map(|w| {
                        // Multiword entries contribute one unit per word.
                        let parts: Vec<String> =
                            w.lemma.split_whitespace().map(String::from).collect();
                        if parts.len() <= 1 {
                            vec![(w.lemma.clone(), w.from, w.to)]
                        } else {
                            parts.into_iter().map(|p| (p, w.from, w.to)).collect()
                        }
                    })
                    .collect(),
                _ => s
                    .tokens
                    .iter()
                    .map(|t| (t.form.to_lowercase(), t.from, t.to))
                    .collect(),
            };
            // Overlapping matches of one rule: the longest wins ("a myriad
            // of" over "myriad").
            let mut found: Vec<(usize, usize, &Vec<String>)> = Vec::new();
            for item in items {
                if item.is_empty() || item.len() > units.len() {
                    continue;
                }
                for start in 0..=units.len() - item.len() {
                    if (0..item.len()).all(|k| units[start + k].0 == item[k]) {
                        found.push((start, item.len(), item));
                    }
                }
            }
            found.sort_by_key(|&(start, len, _)| (std::cmp::Reverse(len), start));
            let mut used = vec![false; units.len()];
            let mut kept = Vec::new();
            for (start, len, item) in found {
                if used[start..start + len].iter().any(|&u| u) {
                    continue;
                }
                used[start..start + len].iter_mut().for_each(|u| *u = true);
                kept.push((start, len, item));
            }
            kept.sort_by_key(|&(start, ..)| start);
            for (start, len, item) in kept {
                let from = units[start].1;
                let to = units[start + len - 1].2;
                let text: String = s
                    .original
                    .chars()
                    .skip(from)
                    .take(to.saturating_sub(from))
                    .collect();
                let mut d = self.diag(a, si, from, to, &text);
                if let Some(rep) = replace.get(&item.join(" ")) {
                    d.replacement = Some(match_case(&text, rep));
                    d.message = d.message.replace("{replacement}", rep);
                }
                out.push(d);
            }
        }
    }
}

impl Rule {
    fn run_consistency(
        &self,
        a: &Analysis,
        prefer: Option<Variety>,
        prefer_suffix: Option<Variety>,
        out: &mut Vec<Diagnostic>,
    ) {
        // Words specific to one variety: (sentence, token, variety, other).
        let mut found: Vec<(usize, usize, Variety, &str)> = Vec::new();
        let table = variants();
        for (si, s) in a.sentences.iter().enumerate() {
            for (ti, t) in s.tokens.iter().enumerate() {
                let w = &t.form;
                // Skip acronyms and inline code.
                let upper = w.chars().filter(|c| c.is_uppercase()).count();
                if upper > 1
                    || s.text
                        .chars()
                        .skip(t.from)
                        .take(t.to - t.from)
                        .all(|c| c == 'x')
                {
                    continue;
                }
                for (v, other) in table.get(&w.to_lowercase()).into_iter().flatten() {
                    found.push((si, ti, *v, other));
                }
            }
        }
        // The variety used most; ties go to the one used first.
        let dominant = |x: Variety, y: Variety| -> Option<Variety> {
            let count = |v| found.iter().filter(|f| f.2 == v).count();
            let (nx, ny) = (count(x), count(y));
            if nx == 0 || ny == 0 {
                return None;
            }
            Some(match nx.cmp(&ny) {
                std::cmp::Ordering::Greater => x,
                std::cmp::Ordering::Less => y,
                std::cmp::Ordering::Equal => found.iter().find(|f| f.2 == x || f.2 == y)?.2,
            })
        };
        let opposite = |v| match v {
            Variety::Us => Variety::Gb,
            Variety::Gb => Variety::Us,
            Variety::Ise => Variety::Ize,
            Variety::Ize => Variety::Ise,
        };
        let us_count = found.iter().filter(|f| f.2 == Variety::Us).count();
        let gb_count = found.iter().filter(|f| f.2 == Variety::Gb).count();
        let dialect = prefer
            .filter(|&p| found.iter().any(|f| f.2 == opposite(p)))
            .or_else(|| dominant(Variety::Us, Variety::Gb));
        // -ise against -ize matters only in British spelling.
        let british = match prefer {
            Some(p) => p == Variety::Gb,
            None => gb_count > 0 && gb_count >= us_count,
        };
        let suffix = if british {
            prefer_suffix
                .filter(|&p| found.iter().any(|f| f.2 == opposite(p)))
                .or_else(|| dominant(Variety::Ise, Variety::Ize))
        } else {
            None
        };
        let mut flagged = std::collections::HashSet::new();
        for want in [dialect, suffix].into_iter().flatten() {
            for &(si, ti, v, other) in &found {
                if v != opposite(want) || !flagged.insert((si, ti)) {
                    continue;
                }
                let t = &a.sentences[si].tokens[ti];
                let rep = match_case(&t.form, other);
                let mut d = self.diag(a, si, t.from, t.to, &t.form);
                d.message = d
                    .message
                    .replace("{replacement}", &rep)
                    .replace("{variety}", v.name())
                    .replace("{dominant}", want.name());
                d.replacement = Some(rep);
                out.push(d);
            }
        }
    }
}

pub use crate::semantics::SemCheck;

/// A grammatical error found by the grammar-error variant of the ERG.
#[derive(Debug, Clone, PartialEq)]
pub struct GrammarError {
    /// The ERG rule, lexical entry or lexical type that names the error.
    pub code: String,
    /// Character span in the sentence.
    pub from: usize,
    pub to: usize,
    pub text: String,
    /// Feedback text, with `$X` filled in.
    pub feedback: String,
}

/// Whether an item of a robust analysis marks an error.
fn is_error_item(name: &str) -> bool {
    !name.starts_with("root_")
        && (name.contains("_rbst")
            || name.contains("_mal")
            || crate::dict::erg_errors().contains_key(name))
}

/// The errors named by the best analysis of the grammar-error variant of
/// the ERG: the reading with the fewest error items (none if some reading
/// has none).
/// More error items than this in the best reading of the grammar-error
/// variant: no named error is reported.
const MAX_ERRORS: usize = 2;

pub fn grammar_errors(s: &crate::Sentence) -> Vec<GrammarError> {
    let Some(p) = &s.mal_parse else {
        return Vec::new();
    };
    let items = |r: &emdysi_parse::Reading| -> Vec<(String, usize, usize)> {
        let mut out = Vec::new();
        for n in &r.nodes {
            if is_error_item(&n.name) {
                out.push((n.name.clone(), n.from, n.to));
            }
        }
        for w in &r.words {
            if is_error_item(&w.le_type) && !out.iter().any(|(c, ..)| *c == w.entry) {
                out.push((w.le_type.clone(), w.from, w.to));
            }
        }
        out
    };
    // Whole-sentence analyses are preferred to fragments; among them, the
    // one that assumes the fewest errors.
    let sentence_root = |r: &str| {
        matches!(
            r,
            "root_decl" | "root_question" | "root_command" | "root_robust_s" | "root_robust_ques"
        ) || emdysi_parse::is_strict(r)
    };
    let pick = |whole: bool| {
        p.readings
            .iter()
            .filter(|r| !whole || sentence_root(&r.root))
            .map(items)
            .min_by_key(|e| e.len())
    };
    let Some(best) = pick(true).or_else(|| pick(false)) else {
        return Vec::new();
    };
    // A reading that needs many corrections is the grammar-error variant
    // making the best of a sentence the grammar could not analyse (long,
    // or with a construction it lacks), not a list of real errors.
    if best.len() > MAX_ERRORS {
        return Vec::new();
    }
    let chars: Vec<char> = s.original.chars().collect();
    // Capitals that are not errors: at the start of the sentence, after a
    // colon or a line break (list labels, verse), and in runs of
    // capitalized words (titles such as "Your Majesty").
    let licensed_capital = |from: usize| {
        let before: String = chars[..from.min(chars.len())].iter().collect();
        let before = before.trim_end_matches([' ', '\t']);
        let prev_word = before
            .rsplit(|c: char| c.is_whitespace())
            .next()
            .unwrap_or("");
        before.trim().is_empty()
            || before.ends_with(':')
            || before.ends_with('\n')
            || prev_word.chars().next().is_some_and(char::is_uppercase)
    };
    let mut out: Vec<GrammarError> = Vec::new();
    for (code, from, to) in best {
        // A capital where none is expected (not the `nocap` errors, a
        // missing capital).
        let wrong_capital = code.contains("cap") && !code.contains("nocap");
        if wrong_capital && licensed_capital(from) {
            continue;
        }
        let text: String = chars
            .get(from..to)
            .map(|c| c.iter().collect())
            .unwrap_or_default();
        let word = text
            .trim_end_matches(|c: char| c.is_ascii_punctuation())
            .trim();
        let feedback = match crate::dict::erg_errors().get(&code) {
            Some(e) => e.feedback.replace("$X", &format!("'{word}'")),
            None => format!("Possible grammatical error near '{word}'."),
        };
        let e = GrammarError {
            code,
            from,
            to: from + word.chars().count().max(1),
            text: word.to_string(),
            feedback,
        };
        if !out
            .iter()
            .any(|o| o.feedback == e.feedback && o.from == e.from)
        {
            out.push(e);
        }
    }
    // When the grammar has a strict analysis that treats some word as
    // unknown (re-parsed for wrong forms such as "buyed"), only errors on
    // those words count: elsewhere the strict analysis stands.
    if s.strict() {
        let unknown: Vec<(usize, usize)> = s
            .best()
            .map(|r| {
                r.words
                    .iter()
                    .filter(|w| w.generic)
                    .map(|w| (w.from, w.to))
                    .collect()
            })
            .unwrap_or_default();
        out.retain(|e| unknown.iter().any(|&(f, t)| e.from < t && f < e.to));
    }
    out
}

/// Give `rep` the capitalization pattern of `like`.
fn match_case(like: &str, rep: &str) -> String {
    if like.chars().next().is_some_and(char::is_uppercase) {
        let mut c = rep.chars();
        c.next()
            .map(|f| f.to_uppercase().chain(c).collect())
            .unwrap_or_default()
    } else {
        rep.to_string()
    }
}

/// Damerau-Levenshtein distance (optimal string alignment).
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.to_lowercase().chars().collect();
    let b: Vec<char> = b.to_lowercase().chars().collect();
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            d[i][j] = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[a.len()][b.len()]
}

/// Keyboard neighbours on a QWERTY layout.
fn adjacent_keys(a: char, b: char) -> bool {
    const ROWS: [&str; 3] = ["qwertyuiop", "asdfghjkl", "zxcvbnm"];
    let pos = |c: char| {
        ROWS.iter()
            .enumerate()
            .find_map(|(r, row)| row.find(c).map(|i| (r as i32, i as i32)))
    };
    match (pos(a), pos(b)) {
        (Some((r1, c1)), Some((r2, c2))) => (r1 - r2).abs() <= 1 && (c1 - c2).abs() <= 1,
        _ => false,
    }
}

/// How plausible it is that `typed` is a typo for `target` (lower is more
/// plausible), for candidates one edit away.
fn typo_cost(typed: &str, target: &str) -> f64 {
    let a: Vec<char> = typed.chars().collect();
    let b: Vec<char> = target.chars().collect();
    let prefix = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    if a.len() == b.len() {
        if prefix + 1 < a.len()
            && a[prefix] == b[prefix + 1]
            && a[prefix + 1] == b[prefix]
            && a[prefix + 2..] == b[prefix + 2..]
        {
            return 0.5; // transposition
        }
        return if adjacent_keys(a[prefix], b[prefix]) {
            0.8
        } else {
            1.0
        };
    }
    // One letter missing from (or added to) what was typed: cheap when it
    // doubles or undoubles a letter.
    let (long, i) = if b.len() > a.len() {
        (&b, prefix)
    } else {
        (&a, prefix)
    };
    let c = long[i];
    let doubled = (i > 0 && long[i - 1] == c) || long.get(i + 1) == Some(&c);
    if doubled { 0.6 } else { 0.9 }
}

/// Spelling suggestions: listed or grammar-known words one edit away (two
/// if none), most plausible and most common first. The flag says whether
/// the first suggestion is clearly the intended word.
fn suggestions(erg: &Erg, word: &str, max: usize) -> (Vec<String>, bool) {
    let lower = word.to_lowercase();
    let known = |w: &str| crate::dict::tier(w).is_some() || erg.known_word(w);
    let mut found: Vec<(f64, u8, String)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let e1 = emdysi_parse::edits(&lower);
    for e in &e1 {
        if seen.insert(e.clone()) && known(e) {
            found.push((
                typo_cost(&lower, e),
                crate::dict::tier(e).unwrap_or(70),
                e.clone(),
            ));
        }
    }
    if found.is_empty() && lower.chars().count() <= 12 {
        for a in &e1 {
            for e in emdysi_parse::edits(a) {
                if seen.insert(e.clone()) && crate::dict::tier(&e).is_some() {
                    found.push((2.0, crate::dict::tier(&e).unwrap_or(70), e));
                }
            }
        }
    }
    found.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)).then(x.2.cmp(&y.2)));
    // Confident: a cheap typo (transposition or doubled letter) with no
    // equally cheap rival of similar commonness.
    let confident = match found.as_slice() {
        [first, rest @ ..] => {
            first.0 <= 0.6
                && rest
                    .first()
                    .is_none_or(|r| r.0 > first.0 || r.1 >= first.1 + 15)
        }
        [] => false,
    };
    let out = found
        .into_iter()
        .take(max)
        .map(|(_, _, w)| match_case(word, &w))
        .collect();
    (out, confident)
}
