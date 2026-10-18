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
//!                           # | articles | repeated-word | ambiguity
//!                           # | existence | substitution | adjective-stack
//!                           # | modifier-density
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

/// A matched group changed for a suggestion: a regex replacement on its
/// text, then the form for a Penn Treebank tag (from UniMorph, see
/// [`crate::dict::inflect`]), then a change of case (`startupper`,
/// `startlower`, `allupper`, `alllower`). In a pack, a string of five
/// tab-separated fields: group, case, pattern, replacement, tag (empty
/// for none).
#[derive(Debug, Clone)]
pub struct Transform {
    group: String,
    case: String,
    pattern: Option<Regex>,
    replacement: String,
    tag: String,
}

impl Transform {
    fn parse(spec: &str) -> Result<Transform, String> {
        let f: Vec<&str> = spec.split('\t').collect();
        let [group, case, pattern, replacement, tag] = f[..] else {
            return Err(format!("a transform has five fields: {spec:?}"));
        };
        Ok(Transform {
            group: group.to_string(),
            case: case.to_string(),
            pattern: if pattern.is_empty() {
                None
            } else {
                Some(Regex::new(pattern).map_err(|e| format!("bad transform pattern: {e}"))?)
            },
            replacement: replacement.to_string(),
            tag: tag.to_string(),
        })
    }

    /// The changed text of the group in `m`; `None` if the group did not
    /// match or has no form for the tag.
    fn apply(&self, m: &fancy_regex::Captures<'_, str>) -> Option<String> {
        let mut text = m.name(&self.group)?.as_str().to_string();
        if let Some(p) = &self.pattern {
            text = p.replace_all(&text, self.replacement.as_str()).into_owned();
        }
        if !self.tag.is_empty() {
            let inflected = crate::dict::inflect(&text, &self.tag)?;
            text = if text.chars().next().is_some_and(char::is_uppercase) {
                upper_first(&inflected)
            } else {
                inflected
            };
        }
        Some(match self.case.as_str() {
            "startupper" => upper_first(&text),
            "startlower" => {
                let mut c = text.chars();
                c.next()
                    .map(|f| f.to_lowercase().chain(c).collect())
                    .unwrap_or_default()
            }
            "allupper" => text.to_uppercase(),
            "alllower" => text.to_lowercase(),
            _ => text,
        })
    }
}

fn upper_first(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// A template with `{{i}}` (transform `i` of `ts`) and `${name}` (a
/// group) filled from `m`; `None` if a transform it uses has no result.
fn fill(template: &str, m: &fancy_regex::Captures<'_, str>, ts: &[Transform]) -> Option<String> {
    let mut t = template.to_string();
    for (i, x) in ts.iter().enumerate() {
        let key = format!("{{{{{i}}}}}");
        if t.contains(&key) {
            t = t.replace(&key, &x.apply(m)?.replace('$', "$$"));
        }
    }
    let mut e = String::new();
    m.expand(&t, &mut e);
    Some(e)
}

/// A part-of-speech condition on a named group of a `regex` rule.
#[derive(Debug, Clone)]
pub struct TagCheck {
    group: String,
    tag: Regex,
    negate: bool,
}

/// A pattern compiled when first used.
#[derive(Debug, Clone)]
pub struct LazyRegex {
    src: String,
    re: std::sync::OnceLock<Option<Regex>>,
}

impl LazyRegex {
    fn new(src: String) -> Self {
        LazyRegex {
            src,
            re: std::sync::OnceLock::new(),
        }
    }

    /// Compile now (once), with the error if the pattern is bad.
    fn compile(&self) -> Result<&Regex, String> {
        if let Some(r) = self.re.get() {
            return r.as_ref().ok_or_else(|| "bad pattern".to_string());
        }
        let r = Regex::new(&self.src).map_err(|e| e.to_string());
        let _ = self.re.set(r.as_ref().ok().cloned());
        r.map(|_| self.re.get().unwrap().as_ref().unwrap())
    }

    /// The compiled pattern; `None` if it does not compile.
    pub fn get(&self) -> Option<&Regex> {
        self.compile().ok()
    }
}

#[derive(Debug, Clone)]
pub enum Kind {
    /// A question for a decision model about each sentence or block
    /// (skipped without a model; see [`crate::decisions`]).
    Decide(crate::decisions::Decide),
    /// Words or phrases, matched on lemmas (from the parse) or surface
    /// forms. `replace` maps a listed item to an automatic fix.
    Words {
        items: Vec<Vec<String>>,
        on: MatchOn,
        replace: HashMap<String, String>,
    },
    /// A regular expression over the sentence text. Sentences that match
    /// `unless` (e.g. a citation) are skipped. A group named `m` narrows
    /// the finding to its span (the rest of the match is context).
    /// `suggest` lists alternatives (expanded like `replace`, `${name}`
    /// for groups); with `expand` the message is expanded too. With
    /// `needs`, a sentence is tried only if it contains one of those words
    /// (lowercase; see [`crate::Sentence::mentions`]), matches inside a
    /// URL, file path or inline code are skipped (they are about words),
    /// and the
    /// patterns compile when one first does: a pack
    /// of a thousand rules then costs little to load or run.
    Regex {
        re: LazyRegex,
        needs: Vec<String>,
        replace: Option<String>,
        unless: Option<LazyRegex>,
        suggest: Vec<String>,
        expand: bool,
        /// Part-of-speech conditions on named groups (`tags`, `not_tags`:
        /// `"t2=NN.*"`): the group must cover a word of the best reading
        /// whose Penn Treebank tag (see [`crate::tags`]) matches, or (with
        /// `negate`) none that does. A rule with conditions needs a parse.
        tags: Vec<TagCheck>,
        /// Changes to a matched group for suggestions and messages,
        /// written `{{0}}`, `{{1}}`... in templates (see [`Transform`]).
        transforms: Vec<Transform>,
    },
    /// The findings of other rules (ids starting with one of `of`), when a
    /// document has at least `min` of them and more than `max` per `per`
    /// words: one finding for the document. A single tell is weak
    /// evidence; their density is the signal.
    TellDensity {
        of: Vec<String>,
        per: f64,
        max: f64,
        min: usize,
    },
    /// A sentence matching `first` followed, in the same paragraph, by one
    /// matching `next` (|Leadership isn't about control. It's about
    /// trust.|).
    SentencePair { first: Regex, next: Regex },
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
        /// With a decision model, the probability at which its choice
        /// among several suggestions becomes the fix (see `assist`).
        model_threshold: f64,
    },
    /// Grammar corrections the strict grammar accepts and a decision model
    /// prefers with probability `threshold` or more (see `assist`); without
    /// a model, nothing.
    AssistedGrammar { threshold: f64, screen: Option<f64> },
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
    /// *a* before a vowel sound, *an* before a consonant sound.
    Articles,
    /// A function word written twice, or two different articles in a row
    /// (`articles`), in a sentence without a full analysis.
    RepeatedWord { articles: bool },
    /// Sentences with more than one meaning: a second interpretation keeps
    /// at least `min_share` of the probability.
    Ambiguity { min_share: f64 },
    /// Noun-noun compounds of `min` to `max` nouns.
    NounStack { min: usize, max: usize },
    /// Nouns that carry `min` or more adjectives.
    AdjectiveStack { min: usize },
    /// Sentences in which adjectives and descriptive adverbs make up at
    /// least `ratio` of the words (and number at least `min`).
    ModifierDensity { min: usize, ratio: f64 },
    /// Any of a list of patterns (Vale's `existence`): `tokens` are joined
    /// into one pattern, between word boundaries unless `nonword`.
    Existence { re: Regex, exceptions: Vec<String> },
    /// Patterns with preferred replacements (Vale's `substitution`): `swap`
    /// maps a pattern to its replacement; alternatives are separated by
    /// `|`. Replacements are suggestions, automatic fixes only with
    /// `fix = true` and a single alternative.
    Substitution {
        /// Pattern, replacements, and the pattern's text when it is a
        /// plain phrase (lowercase): sentences without it are skipped.
        swaps: Vec<(Regex, Vec<String>, Option<String>)>,
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
    /// A pack holding only glossary concepts.
    pub fn from_concepts(concepts: Vec<Concept>) -> Pack {
        Pack {
            name: "glossary".into(),
            description: String::new(),
            rules: Vec::new(),
            concepts,
        }
    }

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
                            .flat_map(split_contraction)
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
                                let k: Vec<String> = k
                                    .to_lowercase()
                                    .split_whitespace()
                                    .flat_map(split_contraction)
                                    .collect();
                                v.as_str().map(|s| (k.join(" "), s.to_string()))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                Kind::Words { items, on, replace }
            }
            "regex" => {
                let pat = get_str(t, "pattern").ok_or_else(|| err("regex rule needs `pattern`"))?;
                let needs: Vec<String> = get_list(t, "needs")
                    .iter()
                    .map(|w| w.to_lowercase())
                    .collect();
                let lazy = |src: String, what: &str| {
                    let r = LazyRegex::new(src);
                    // Without `needs`, a bad pattern is an error now.
                    if needs.is_empty() {
                        r.compile().map_err(|e| err(&format!("bad {what}: {e}")))?;
                    }
                    Ok::<_, PackError>(r)
                };
                let re = lazy(pat, "pattern")?;
                let unless = get_str(t, "unless")
                    .map(|u| lazy(u, "`unless` pattern"))
                    .transpose()?;
                Kind::Regex {
                    re,
                    needs,
                    replace: get_str(t, "replace"),
                    unless,
                    suggest: get_list(t, "suggest"),
                    expand: t.get("expand").and_then(Value::as_bool).unwrap_or(false),
                    tags: [("tags", false), ("not_tags", true)]
                        .iter()
                        .flat_map(|&(key, negate)| {
                            get_list(t, key).into_iter().map(move |c| (c, negate))
                        })
                        .map(|(c, negate)| {
                            let (group, tag) = c
                                .split_once('=')
                                .ok_or_else(|| err(&format!("bad tag condition {c:?}")))?;
                            Ok(TagCheck {
                                group: group.to_string(),
                                tag: Regex::new(&format!("^(?:{tag})$"))
                                    .map_err(|e| err(&format!("bad tag pattern: {e}")))?,
                                negate,
                            })
                        })
                        .collect::<Result<_, PackError>>()?,
                    transforms: get_list(t, "transforms")
                        .iter()
                        .map(|x| Transform::parse(x).map_err(|e| err(&e)))
                        .collect::<Result<_, PackError>>()?,
                }
            }
            "tell-density" => Kind::TellDensity {
                of: get_list(t, "of"),
                per: num("per").unwrap_or(100.0),
                max: num("max").ok_or_else(|| err("tell-density rule needs `max`"))?,
                min: num("min").unwrap_or(5.0) as usize,
            },
            "sentence-pair" => {
                let re = |key: &str| -> Result<Regex, _> {
                    let pat = get_str(t, key)
                        .ok_or_else(|| err(&format!("sentence-pair rule needs `{key}`")))?;
                    Regex::new(&pat).map_err(|e| err(&format!("bad `{key}` pattern: {e}")))
                };
                Kind::SentencePair {
                    first: re("pattern")?,
                    next: re("next")?,
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
                model_threshold: num("model_threshold").unwrap_or(0.8),
            },
            "assisted-grammar" => Kind::AssistedGrammar {
                threshold: num("threshold").unwrap_or(0.8),
                screen: num("screen"),
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
                    "vacuous" => SemCheck::Vacuous,
                    "aphorism" => SemCheck::Aphorism,
                    "restatement" => SemCheck::Restatement,
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
                    "one-line-paragraphs" => StructureCheck::OneLineParagraphs {
                        max_words: n("max_words", 8.0),
                        min: n("min", 3.0),
                        share: num("share").unwrap_or(0.4),
                    },
                    "bold-lead-ins" => StructureCheck::BoldLeadIns {
                        min: n("min", 3.0),
                        share: num("share").unwrap_or(0.5),
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
            "articles" => Kind::Articles,
            "ambiguity" => Kind::Ambiguity {
                min_share: num("min_share").unwrap_or(0.25),
            },
            "repeated-word" => Kind::RepeatedWord {
                articles: matches!(t.get("articles"), Some(Value::Bool(true))),
            },
            "adjective-stack" => Kind::AdjectiveStack {
                min: num("min").unwrap_or(3.0) as usize,
            },
            "modifier-density" => Kind::ModifierDensity {
                min: num("min").unwrap_or(5.0) as usize,
                ratio: num("ratio").unwrap_or(0.3),
            },
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
                        let plain = k
                            .chars()
                            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '\'' | '-'))
                            .then(|| k.to_lowercase());
                        swaps.push((
                            re,
                            rep.split('|').map(|r| r.trim().to_string()).collect(),
                            plain,
                        ));
                    }
                    Kind::Substitution {
                        swaps,
                        fix: t.get("fix").and_then(Value::as_bool).unwrap_or(false),
                    }
                }
            }
            "decide" => {
                let options = get_list(t, "options");
                if options.len() < 2 {
                    return Err(err("`options` needs at least two answers"));
                }
                let flag = get_str(t, "flag").ok_or_else(|| err("`flag` is required"))?;
                let flag = options
                    .iter()
                    .position(|o| *o == flag)
                    .ok_or_else(|| err("`flag` must be one of `options`"))?;
                let unit = get_str(t, "unit").unwrap_or_else(|| "sentence".into());
                Kind::Decide(crate::decisions::Decide {
                    question: get_str(t, "question")
                        .ok_or_else(|| err("`question` is required"))?,
                    options,
                    flag,
                    threshold: num("threshold").unwrap_or(0.8),
                    unit: crate::decisions::Unit::parse(&unit)
                        .ok_or_else(|| err(&format!("unknown unit {unit:?}")))?,
                    min_words: num("min_words").unwrap_or(1.0) as usize,
                })
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
            needs_model: false,
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
            needs_model: false,
        }
    }

    /// The decision model's choice among the suggestions of spelling
    /// findings that have no fix (see `assist::spelling`).
    fn choose_spellings(
        &self,
        erg: &Erg,
        a: &Analysis,
        ask: &mut dyn crate::decisions::Ask,
        threshold: f64,
        found: &mut [Diagnostic],
    ) {
        for d in found.iter_mut() {
            let Some(si) = d.sentence else { continue };
            if d.replacement.is_some() || (d.suggestions.len() < 2 && !d.needs_model) {
                continue;
            }
            let Some(t) = a.sentences[si]
                .tokens
                .iter()
                .find(|t| a.source_range(si, t.from, t.to) == d.range)
            else {
                continue;
            };
            // A finding to confirm offers the word as written too: the model
            // may know it (|distroless|, |mebibytes|).
            let written: String = a.sentences[si]
                .original
                .chars()
                .skip(t.from)
                .take(t.to - t.from)
                .collect();
            let options: Vec<String> = if d.needs_model {
                std::iter::once(written.clone())
                    .chain(d.suggestions.iter().cloned())
                    .collect()
            } else {
                d.suggestions.clone()
            };
            let Some((rep, p)) =
                crate::assist::spelling(erg, a, (si, t.from, t.to), &options, ask, threshold)
            else {
                continue;
            };
            if rep == written {
                continue;
            }
            d.needs_model = false;
            if let Some(first) = d.suggestions.first() {
                d.message = d.message.replace(
                    &format!(" Did you mean '{first}'?"),
                    &format!(" Did you mean '{rep}'?"),
                );
            }
            d.message.push_str(&format!(
                " (chosen in context by the model, {:.0}%)",
                p * 100.0
            ));
            d.suggestions.retain(|x| !x.eq_ignore_ascii_case(&rep));
            d.suggestions.insert(0, rep.clone());
            d.replacement = Some(rep);
        }
    }

    /// A `tell-density` rule over the findings of the other rules.
    pub fn run_density(&self, a: &Analysis, found: &[Diagnostic], out: &mut Vec<Diagnostic>) {
        let Kind::TellDensity { of, per, max, min } = &self.kind else {
            return;
        };
        let tells = found
            .iter()
            .filter(|d| d.rule != self.id && of.iter().any(|p| d.rule.starts_with(p.as_str())))
            .count();
        let words: usize = a.sentences.iter().map(|s| s.word_count()).sum();
        if words == 0 || tells < *min {
            return;
        }
        let rate = tells as f64 * per / words as f64;
        if rate <= *max || a.sentences.is_empty() {
            return;
        }
        let n = a.sentences[0].original.trim_end().chars().count();
        let mut d = self.diag(a, 0, 0, n, "");
        d.message = d
            .message
            .replace("{count}", &tells.to_string())
            .replace("{words}", &words.to_string())
            .replace("{rate}", &format!("{rate:.1}"))
            .replace("{per}", &format!("{per}"));
        out.push(d);
    }

    /// Run the rule over an analysis, with the glossary of all loaded packs.
    pub fn run(&self, erg: &Erg, a: &Analysis, g: &Glossary, out: &mut Vec<Diagnostic>) {
        self.run_with(erg, a, g, None, out);
    }

    /// [`Rule::run`], with a decision model for `decide` rules.
    pub fn run_with(
        &self,
        erg: &Erg,
        a: &Analysis,
        g: &Glossary,
        ask: Option<&mut (dyn crate::decisions::Ask + '_)>,
        out: &mut Vec<Diagnostic>,
    ) {
        let mut found = Vec::new();
        if let Kind::Decide(d) = &self.kind {
            if let Some(ask) = ask {
                let hits = crate::decisions::run_decide(d, a, ask);
                found.extend(hits.into_iter().map(|h| self.hit_diag(h)));
            }
        } else if let Kind::AssistedGrammar { threshold, screen } = &self.kind {
            if let Some(ask) = ask {
                for c in crate::assist::grammar(erg, a, ask, *threshold, *screen) {
                    let mut d = self.diag(a, c.sentence, c.from, c.to, &c.text);
                    d.message = d
                        .message
                        .replace("{replacement}", &c.replacement)
                        .replace("{percent}", &format!("{:.0}", c.probability * 100.0));
                    d.replacement = Some(c.replacement);
                    found.push(d);
                }
            }
        } else {
            self.run_all(erg, a, g, &mut found);
            let with_model = ask.is_some();
            if let (
                Kind::Spelling {
                    model_threshold, ..
                },
                Some(ask),
            ) = (&self.kind, ask)
            {
                self.choose_spellings(erg, a, ask, *model_threshold, &mut found);
            }
            // What only a model could confirm is not reported without one
            // (`choose_spellings` clears the mark of what it confirms).
            // ... except, without a model, a misspelling with a single
            // candidate that it only drops or adds letters to
            // (|admistrators|, |neccesary|, |securly|): a word only
            // substitutions make a known word of is more often a name or
            // term (|grafana|, |mebibytes|).
            for d in found.iter_mut().filter(|_| !with_model) {
                let w = &a.source[d.range.clone()];
                if d.needs_model
                    && d.suggestions.len() == 1
                    && w.chars().count() >= 6
                    && w.chars().all(|c| !c.is_uppercase())
                    && indels_only(w, &d.suggestions[0])
                {
                    d.needs_model = false;
                }
            }
            found.retain(|d| !d.needs_model);
        }
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
            Kind::Articles => Some(crate::articles::run_articles(a)),
            Kind::RepeatedWord { articles } => Some(crate::repeats::run_repeats(a, *articles)),
            Kind::AdjectiveStack { min } => Some(crate::modifiers::run_adjective_stacks(a, *min)),
            Kind::ModifierDensity { min, ratio } => {
                Some(crate::modifiers::run_modifier_density(a, *min, *ratio))
            }
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
            Kind::Words { items, on, replace } => self.run_words(erg, a, items, *on, replace, out),
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
                    let lower = s.text.to_lowercase();
                    for (re, reps, plain) in swaps {
                        if plain.as_ref().is_some_and(|p| !lower.contains(p.as_str())) {
                            continue;
                        }
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
                suggest,
                expand,
                needs,
                tags,
                transforms,
            } => {
                for (si, s) in a.sentences.iter().enumerate() {
                    if !tags.is_empty() && s.best().is_none() {
                        continue;
                    }
                    if !needs.is_empty() && !needs.iter().any(|w| s.mentions(w)) {
                        continue;
                    }
                    let Some(re) = re.get() else { continue };
                    if unless
                        .as_ref()
                        .and_then(LazyRegex::get)
                        .is_some_and(|u| u.is_match(&s.original).unwrap_or(false))
                    {
                        continue;
                    }
                    for m in re.captures_iter(s.original.as_str()).flatten() {
                        let m0 = m.name("m").or_else(|| m.get(0)).unwrap();
                        let from = s.original[..m0.start()].chars().count();
                        let to = from + m0.as_str().chars().count();
                        // Word rules skip URLs, paths and inline code
                        // (masked in `s.text`): |the `Some` variant|.
                        if !needs.is_empty()
                            && (in_url_or_path(&s.original, m0.start(), m0.end())
                                || s.text
                                    .chars()
                                    .zip(s.original.chars())
                                    .skip(from)
                                    .take(to - from)
                                    .any(|(t, o)| t != o))
                        {
                            continue;
                        }
                        if !tags.iter().all(|c| tag_holds(s, &m, c)) {
                            continue;
                        }
                        let mut d = self.diag(a, si, from, to, m0.as_str());
                        if let Some(rep) = replace {
                            d.replacement = fill(rep, &m, transforms);
                        }
                        d.suggestions = suggest
                            .iter()
                            .filter_map(|t| fill(t, &m, transforms))
                            .collect();
                        if *expand {
                            // Without a transform's result, the group as
                            // written.
                            let msg = fill(&self.message, &m, transforms).unwrap_or_else(|| {
                                let mut t = self.message.clone();
                                for (i, x) in transforms.iter().enumerate() {
                                    t = t.replace(
                                        &format!("{{{{{i}}}}}"),
                                        &format!("${{{}}}", x.group),
                                    );
                                }
                                fill(&t, &m, &[]).unwrap_or_default()
                            });
                            d.message = msg.replace("{match}", m0.as_str());
                        }
                        out.push(d);
                    }
                }
            }
            // Run after the other rules: see `Rule::run_density`.
            Kind::TellDensity { .. } => {}
            Kind::SentencePair { first, next } => {
                for si in 0..a.sentences.len().saturating_sub(1) {
                    let (s, n) = (&a.sentences[si], &a.sentences[si + 1]);
                    if n.block != s.block || !next.is_match(&n.original).unwrap_or(false) {
                        continue;
                    }
                    if let Some(m) = first.find(&s.original).ok().flatten() {
                        let from = s.original[..m.start()].chars().count();
                        let to = s.original.chars().count();
                        let text: String = s.original.chars().skip(from).collect();
                        out.push(self.diag(a, si, from, to, text.trim_end()));
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
            Kind::Spelling {
                min_length, ignore, ..
            } => {
                // A word the document uses more than once is one of its
                // terms ("kubelet"), not a typo.
                let mut uses: HashMap<String, usize> = HashMap::new();
                for s in &a.sentences {
                    for t in &s.tokens {
                        *uses.entry(t.form.to_lowercase()).or_default() += 1;
                    }
                }
                // Words the document writes in lower case: a word it uses
                // only capitalized is a name (|Aaron Turon|), not the word
                // a typo was meant for.
                let plain: std::collections::HashSet<&str> = a
                    .sentences
                    .iter()
                    .flat_map(|s| s.tokens.iter())
                    .map(|t| t.form.as_str())
                    .filter(|f| f.chars().all(|c| !c.is_uppercase()))
                    .collect();
                // People the document mentions (|@tiif|, |Comment by
                // @epage|): the same word without the |@| is their name.
                let mentioned: std::collections::HashSet<String> = a
                    .source
                    .split(|c: char| !(c.is_alphanumeric() || c == '@' || c == '-' || c == '_'))
                    .filter_map(|w| w.strip_prefix('@'))
                    .map(str::to_lowercase)
                    .collect();
                // Words the document writes as code (|`notin`|, a fenced
                // block) are its identifiers, wherever else they appear.
                let in_code = code_words(&a.source);
                // Words the document capitalizes after the start of a
                // sentence are names.
                let names: std::collections::HashSet<&str> = a
                    .sentences
                    .iter()
                    .flat_map(|s| s.tokens.iter().skip(1))
                    .filter(|t| t.form.chars().next().is_some_and(char::is_uppercase))
                    .map(|t| t.form.as_str())
                    .collect();
                for (si, s) in a.sentences.iter().enumerate() {
                    for (ti, t) in s.tokens.iter().enumerate() {
                        // A capitalized first word is checked in lower case
                        // ("Althought", "Insetad,"), unless the document uses
                        // it as a name, or it reads like one: followed by
                        // another capitalized word.
                        let lowered;
                        let name_like = s
                            .tokens
                            .get(1)
                            .is_some_and(|n| n.form.chars().next().is_some_and(char::is_uppercase));
                        // A capitalized word in a title-cased run of common
                        // words (|Node Shutdown Mananger|) is not a name:
                        // a possible typo, for the model to confirm.
                        let title_case = |f: &str| {
                            f.chars().next().is_some_and(char::is_uppercase)
                                && f.chars().skip(1).all(char::is_lowercase)
                        };
                        let common_title = |n: Option<&emdysi_parse::InputToken>| {
                            n.is_some_and(|n| {
                                title_case(&n.form)
                                    && crate::dict::tier(&n.form.to_lowercase())
                                        .is_some_and(|t| t <= 50)
                            })
                        };
                        let in_title = ti > 0
                            && t.form.chars().count() >= 5
                            && title_case(&t.form)
                            && (common_title(s.tokens.get(ti - 1))
                                || common_title(s.tokens.get(ti + 1)));
                        // A three-letter word one edit from one of the
                        // commonest words (|wil|, |tha|, |Nex,| at the start)
                        // is a possible typo, for the model to confirm.
                        let short_typo = {
                            let l = t.form.to_lowercase();
                            l.chars().count() + 1 == *min_length
                                && l.chars().all(|c| c.is_ascii_lowercase())
                                && (ti == 0 || t.form == l)
                                && crate::dict::tier(&l).is_none()
                                && !erg.known_word(&l)
                                && !crate::dict::accepted(&l)
                                && emdysi_parse::edits(&l)
                                    .iter()
                                    .any(|e| crate::dict::tier(e).is_some_and(|t| t <= SHORT_TIER))
                        };
                        let w: &String = if in_title
                            || short_typo && ti == 0
                            || ti == 0
                            // Short capitalized words are often names
                            // (|Tage has started|, |Nuage is a platform|).
                            && t.form.chars().count() >= 7
                            && t.form.chars().next().is_some_and(char::is_uppercase)
                            && t.form.chars().skip(1).all(char::is_lowercase)
                            && !names.contains(t.form.as_str())
                            && !name_like
                        {
                            lowered = t.form.to_lowercase();
                            &lowered
                        } else {
                            &t.form
                        };
                        // Only plain lower-case words: names, acronyms, code
                        // and numbers are out of scope.
                        if w.chars().count() < *min_length && !short_typo
                            || !w.chars().all(|c| c.is_lowercase() || c == '\'' || c == '-')
                            || w.contains("--")
                            // Inline code, or a word around it (|`let`s|).
                            || s.text
                                .chars()
                                .zip(s.original.chars())
                                .skip(t.from)
                                .take(t.to - t.from)
                                .any(|(m, o)| m != o)
                            || ignore.contains(w)
                            // ... and the plural of an ignored word (|zulips|).
                            || w.strip_suffix('s').is_some_and(|b| ignore.iter().any(|i| i == b))
                            || mentioned.contains(w.as_str())
                            || in_code.contains(w.to_lowercase().as_str())
                            || g.knows_word(w)
                        {
                            continue;
                        }
                        let known = |p: &str| {
                            crate::dict::tier(p).is_some()
                                || crate::dict::accepted(p)
                                || erg.known_word(p)
                                || crate::dict::inflection_of_known(p)
                        };
                        let parts: Vec<&str> = w
                            .split(['-', '\'', '’'])
                            .filter(|p| !p.is_empty())
                            .collect();
                        if known(w)
                            || parts.iter().all(|p| known(p))
                            || crate::in_code_like(&s.text, t.from, t.to)
                        {
                            continue;
                        }
                        // A misspelling codespell lists (from English
                        // Wikipedia's lists of common misspellings) is one,
                        // however far from its correction and however often
                        // the document repeats it; its fix is applied when
                        // codespell gives only one.
                        if let Some((fixes, sure)) = crate::dict::codespell(w) {
                            let mut d = self.diag(a, si, t.from, t.to, w);
                            d.message = d
                                .message
                                .replace("{suggestion}", &format!(" Did you mean '{}'?", fixes[0]));
                            d.suggestions = fixes.to_vec();
                            if sure {
                                d.replacement = Some(fixes[0].clone());
                            }
                            out.push(d);
                            continue;
                        }
                        // A known word plus an affix ("promptability") is a
                        // coinage, not a misspelling: see `coined-words`.
                        // A listed word of four or more letters plus an affix
                        // is a misspelling only if a real inflection of
                        // that base is one edit away ("runing" for
                        // "running"); otherwise a coinage ("liveness",
                        // "mortifications").
                        let mut derived_typo = false;
                        if let Some((base, _)) =
                            crate::terms::novel_derivation(erg, w).filter(|(b, _)| {
                                b.chars().count() >= 4 && crate::dict::tier(b).is_some()
                            })
                        {
                            // (A common word one edit away is no proof of a
                            // typo here: |destructures| is one edit from
                            // |restructures|, |liveness| from |aliveness|.)
                            let inflections = erg.inflections(&base);
                            // ... or a listed word one edit away, inside the
                            // word, is formed on the same base (|assignement|
                            // for |assignment|, |securly| for |securely|):
                            // |aliveness|, one edit from |liveness|, is not.
                            let stem: String = base.chars().take(4).collect();
                            let by_inflection =
                                inflections.iter().any(|f| edit_distance(w, f) == 1);
                            let by_derivation = suggestions(erg, w, 5).0.iter().any(|x| {
                                let x = x.to_lowercase();
                                // The edit inside the word: one at its
                                // end is another affix (|flutterings|,
                                // |finalish|).
                                let same_end = w
                                    .chars()
                                    .rev()
                                    .zip(x.chars().rev())
                                    .take_while(|(a, b)| a == b)
                                    .count()
                                    >= 2;
                                edit_distance(w, &x) == 1
                                    && same_end
                                    && crate::dict::tier(&x).is_some()
                                    && x.starts_with(&stem)
                            });
                            // ... or a listed word with one letter typed
                            // twice (|languagess|, |valuess|).
                            let doubled = suggestions(erg, w, 5).0.iter().any(|x| {
                                let x = x.to_lowercase();
                                let c: Vec<char> = w.chars().collect();
                                crate::dict::tier(&x).is_some()
                                    && (1..c.len()).any(|i| {
                                        c[i] == c[i - 1]
                                            && c[..i].iter().chain(&c[i + 1..]).collect::<String>()
                                                == x
                                    })
                            });
                            if !by_inflection && !by_derivation && !doubled {
                                continue;
                            }
                            // A rare correct plural can also be one edit from
                            // a listed word (|nestings|, |nestlings|): only
                            // the model can tell.
                            derived_typo = !by_inflection && !doubled;
                        }
                        if uses.get(&w.to_lowercase()).copied().unwrap_or(0) > 1 {
                            continue;
                        }
                        let (mut sugg, confident) = suggestions(erg, w, 5);
                        // A word one edit from a word the document itself
                        // uses (|crytographic| beside |cryptographic|) is
                        // a typo for it, even when that word is too rare
                        // for the word lists' common tiers.
                        let lower = w.to_lowercase();
                        let mut in_document: Vec<String> = emdysi_parse::edits(&lower)
                            .into_iter()
                            .filter(|e| *e != lower && plain.contains(e.as_str()) && known(e))
                            .collect();
                        in_document.sort();
                        in_document.dedup();
                        let by_document = in_document.len() == 1;
                        if by_document {
                            sugg.retain(|x| x.to_lowercase() != in_document[0]);
                            sugg.insert(0, in_document[0].clone());
                        }
                        // A typo is one edit away from the word meant: an
                        // unknown word with no known word that close is a
                        // rare word, not evidence of an error.
                        // ... and the word meant is a listed word, not one the
                        // grammar's morphology derives ("re-" + "sugar" +
                        // "-ing" for "desugaring").
                        // Two edits away is a typo too (|neccesary|,
                        // |certicate|) when the word is long, the word meant
                        // is common, and few common words are that close;
                        // the model then chooses, offered the word as
                        // written too, and it is never a fix on its own.
                        let common = |x: &str| {
                            crate::dict::tier(&x.to_lowercase()).is_some_and(|t| t <= TWO_EDIT_TIER)
                        };
                        // ... and not two known words written as one
                        // (|shockwaves|), a compound rather than a typo.
                        let compound = (3..w.len().saturating_sub(2)).any(|i| {
                            w.is_char_boundary(i)
                                && crate::dict::tier(&w[..i]).is_some()
                                && crate::dict::tier(&w[i..]).is_some()
                        });
                        let two_edits = !compound
                            && w.chars().count() >= TWO_EDIT_LENGTH
                            && sugg
                                .first()
                                .is_some_and(|x| edit_distance(w, x) == 2 && common(x))
                            && sugg
                                .iter()
                                .filter(|x| edit_distance(w, x) <= 2 && common(x))
                                .count()
                                <= TWO_EDIT_CANDIDATES;
                        if !two_edits
                            && !by_document
                            && !sugg.first().is_some_and(|x| {
                                edit_distance(w, x) == 1
                                    && crate::dict::tier(&x.to_lowercase()).is_some()
                            })
                        {
                            continue;
                        }
                        let mut d = self.diag(
                            a,
                            si,
                            t.from,
                            t.to,
                            if in_title || short_typo { &t.form } else { w },
                        );
                        d.message = d.message.replace(
                            "{suggestion}",
                            &sugg
                                .first()
                                .cloned()
                                .map(|x| format!(" Did you mean '{x}'?"))
                                .unwrap_or_default(),
                        );
                        d.needs_model = two_edits || derived_typo || in_title || short_typo;
                        // Fix automatically only when one candidate is clearly
                        // the most plausible typo, or the only one.
                        let single = sugg.len() == 1
                            && edit_distance(w, &sugg[0]) == 1
                            && w.chars().count() >= 4;
                        if !d.needs_model && (confident || single || by_document) {
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
                // Words the document capitalizes after the start of a
                // sentence are names, at the start of one too ("Pod is ...").
                let names: std::collections::HashSet<String> = a
                    .sentences
                    .iter()
                    .flat_map(|s| s.tokens.iter().skip(1))
                    .filter(|t| t.form.chars().next().is_some_and(char::is_uppercase))
                    .map(|t| t.form.clone())
                    .collect();
                for (si, s) in a.sentences.iter().enumerate() {
                    // A sentence that is all bold or underlined is a label
                    // ("**Remote moderated usability testing**."), not a
                    // clause.
                    let r = a.source_range(si, 0, s.original.chars().count());
                    // The opening marker may lie just before the sentence.
                    let start = a.source[..r.start]
                        .char_indices()
                        .rev()
                        .nth(1)
                        .map_or(0, |(i, _)| i);
                    let src = &a.source[start..r.end];
                    let body = src.trim().trim_end_matches(['.', ':', '!', '?']);
                    let body = ["- ", "* ", "+ "]
                        .iter()
                        .find_map(|b| body.strip_prefix(b))
                        .unwrap_or(body)
                        .trim_start();
                    let label = |m: &str| {
                        let inner = body.strip_prefix(m).and_then(|b| b.strip_suffix(m));
                        inner.is_some_and(|i| !i.is_empty() && !i.contains(m))
                    };
                    if label("**") || label("__") {
                        continue;
                    }
                    let chars: Vec<char> = s.original.chars().collect();
                    let span =
                        |from: usize, to: usize| -> String { chars[from..to].iter().collect() };
                    for e in grammar_errors(s) {
                        // Reported only with a correction the grammar
                        // accepts: see `verified_fix`.
                        let Some((from, to, fix)) = verified_fix(erg, s, &e, &names) else {
                            continue;
                        };
                        let mut d = self.diag(a, si, from, to, &span(from, to));
                        // A run-on (see `candidate_fixes`) is told by its
                        // repair: a missing semicolon or a pronoun too many.
                        let feedback = if e.code != "cl-cl_runon_c" {
                            e.feedback.clone()
                        } else if fix.starts_with(';') {
                            "Two clauses run together without punctuation.".to_string()
                        } else {
                            format!("'{}' looks like one word too many.", span(from, to).trim())
                        };
                        d.message = d
                            .message
                            .replace("{feedback}", &feedback)
                            .replace("{code}", &e.code);
                        d.suggestions = vec![fix];
                        out.push(d);
                    }
                    // Analyses that disagree on the error: the sentence is
                    // reported with every correction the grammar accepts,
                    // the likeliest analysis's first.
                    let alts = grammar_error_alternatives(s);
                    let verified: Vec<(&GrammarError, (usize, usize, String))> = alts
                        .iter()
                        .filter_map(|e| Some((e, verified_fix(erg, s, e, &names)?)))
                        .collect();
                    let fixes: Vec<&(usize, usize, String)> =
                        verified.iter().map(|(_, f)| f).collect();
                    // An analysis whose corrections the grammar all
                    // rejects (|His will makes it|) describes no error it
                    // can repair; one for which no correction can be tried
                    // (|How he lived I know not| as a run-on that wants a
                    // comma) may be what the writer meant, and blocks the
                    // claim.
                    // An untried analysis of exactly the words a verified
                    // correction mends (|The cluster info are known|: |are|
                    // as the wrong form of *be*, or as a robust *is* with
                    // nothing to try) is the same error; one of other words
                    // still blocks (|How he lived I know not|).
                    let untried = alts.iter().any(|e| {
                        !verified.iter().any(|(v, _)| std::ptr::eq(*v, e))
                            && !verified
                                .iter()
                                .any(|(v, _)| v.from == e.from && v.to == e.to)
                            // A run-on's few repairs (see `candidate_fixes`)
                            // failing does not rule it out: it may want a
                            // comma.
                            && (e.code == "cl-cl_runon_c"
                                || candidate_fixes(erg, s, e, &names).is_empty())
                    });
                    if fixes.is_empty() || untried {
                        continue;
                    }
                    let from = fixes.iter().map(|f| f.0).min().unwrap_or(0);
                    let to = fixes.iter().map(|f| f.1).max().unwrap_or(0);
                    let mut suggestions: Vec<String> = Vec::new();
                    for &(f, t, rep) in &fixes {
                        let whole = format!("{}{rep}{}", span(from, *f), span(*t, to));
                        if !suggestions.contains(&whole) {
                            suggestions.push(whole);
                        }
                    }
                    let feedback = if suggestions.len() == 1 {
                        verified[0].0.feedback.clone()
                    } else {
                        format!(
                            "This is not grammatical; the grammar accepts {} corrections.",
                            suggestions.len()
                        )
                    };
                    let mut d = self.diag(a, si, from, to, &span(from, to));
                    d.message = d
                        .message
                        .replace("{feedback}", &feedback)
                        .replace("{code}", "alternatives");
                    d.suggestions = suggestions;
                    out.push(d);
                }
            }
            Kind::Ambiguity { min_share } => {
                for (si, s) in a.sentences.iter().enumerate() {
                    let block_kind = a.blocks[s.block].kind;
                    if matches!(
                        block_kind,
                        emdysi_text::blocks::BlockKind::Heading(_)
                            | emdysi_text::blocks::BlockKind::TableCell
                    ) {
                        continue;
                    }
                    let Some((shares, alternative)) = crate::report::ambiguity(s, *min_share)
                    else {
                        continue;
                    };
                    let len = s.original.chars().count();
                    let mut d = self.diag(a, si, 0, len, &s.original);
                    let pct: Vec<String> = shares
                        .iter()
                        .map(|x| format!("{:.0}%", 100.0 * x))
                        .collect();
                    d.message = d
                        .message
                        .replace("{count}", &shares.len().to_string())
                        .replace("{shares}", &pct.join(", "))
                        .replace("{alternative}", &alternative);
                    out.push(d);
                }
            }
            Kind::Grammar => {
                for (si, s) in a.sentences.iter().enumerate() {
                    let Some(p) = &s.parse else { continue };
                    // A named error is reported by `grammar-errors` instead.
                    if !grammar_errors(s).is_empty() || !grammar_error_alternatives(s).is_empty() {
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
        erg: &Erg,
        a: &Analysis,
        items: &[Vec<String>],
        on: MatchOn,
        replace: &HashMap<String, String>,
        out: &mut Vec<Diagnostic>,
    ) {
        for (si, s) in a.sentences.iter().enumerate() {
            // Units: (lemma or surface, from, to): lemmas from the best
            // reading when there is one, and the tokens.
            let surface: Vec<(String, usize, usize)> = s
                .tokens
                .iter()
                .map(|t| (t.form.to_lowercase().replace('\'', "’"), t.from, t.to))
                .collect();
            let lemmas: Option<Vec<(String, usize, usize)>> = match (on, s.best()) {
                (MatchOn::Lemma, Some(r)) => Some(
                    r.words
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
                ),
                _ => None,
            };
            // Matches as (from, to, units, item). A hyphenated item
            // (|ever-evolving|) is matched on the tokens: the parse has its
            // parts as separate words with their own lemmas.
            let mut found: Vec<(usize, usize, usize, &Vec<String>)> = Vec::new();
            for item in items {
                let units = match &lemmas {
                    Some(l) if !item.iter().any(|w| w == "-") => l,
                    _ => &surface,
                };
                if item.is_empty() || item.len() > units.len() {
                    continue;
                }
                for start in 0..=units.len() - item.len() {
                    if (0..item.len()).all(|k| units[start + k].0 == item[k]) {
                        let (from, to) = (units[start].1, units[start + item.len() - 1].2);
                        found.push((from, to, item.len(), item));
                    }
                }
            }
            // Overlapping matches of one rule: the longest wins ("a myriad
            // of" over "myriad").
            found.sort_by_key(|&(from, to, len, _)| (std::cmp::Reverse(len), from, to));
            let mut kept: Vec<(usize, usize, &Vec<String>)> = Vec::new();
            for (from, to, _, item) in found {
                if kept.iter().any(|&(f, t, _)| from < t && f < to) {
                    continue;
                }
                kept.push((from, to, item));
            }
            kept.sort_by_key(|&(from, ..)| from);
            for (from, to, item) in kept {
                let text: String = s
                    .original
                    .chars()
                    .skip(from)
                    .take(to.saturating_sub(from))
                    .collect();
                let mut d = self.diag(a, si, from, to, &text);
                if let Some(rep) = replace.get(&item.join(" ")) {
                    // A word matched by its lemma is replaced in the same
                    // form (|utilizes| by |uses|); without such a form there
                    // is no automatic fix.
                    let lower = text.to_lowercase();
                    let inflected = if on == MatchOn::Lemma
                        && item.len() == 1
                        && !rep.contains(' ')
                        && lower != item[0]
                    {
                        erg.inflect_like(rep, &lower)
                            .into_iter()
                            .find(|f| crate::dict::tier(f).is_some())
                    } else {
                        Some(rep.clone())
                    };
                    let shown = inflected.clone().unwrap_or_else(|| rep.clone());
                    d.replacement = inflected.map(|r| match_case(&text, &r));
                    d.message = d.message.replace("{replacement}", &shown);
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
        // Words the document capitalizes after the start of a sentence are
        // names at the start of one too (|Matt| is not a spelling of |mat|).
        let names: std::collections::HashSet<&str> = a
            .sentences
            .iter()
            .flat_map(|s| s.tokens.iter().skip(1))
            .filter(|t| t.form.chars().next().is_some_and(char::is_uppercase))
            .map(|t| t.form.as_str())
            .collect();
        for (si, s) in a.sentences.iter().enumerate() {
            for (ti, t) in s.tokens.iter().enumerate() {
                let w = &t.form;
                // Skip acronyms, inline code and names: a capitalized word
                // after the start of a sentence ("Matt") is not a spelling of
                // a common word.
                let upper = w.chars().filter(|c| c.is_uppercase()).count();
                if upper > 1
                    || (ti > 0 && w.chars().next().is_some_and(char::is_uppercase))
                    || names.contains(w.as_str())
                    // ... or followed by another capitalized word, like a
                    // name (|Matt Mauer had an idea|).
                    || (ti == 0
                        && w.chars().next().is_some_and(char::is_uppercase)
                        && s.tokens
                            .get(1)
                            .is_some_and(|n| n.form.chars().next().is_some_and(char::is_uppercase)))
                    || s.text
                        .chars()
                        .skip(t.from)
                        .take(t.to - t.from)
                        .all(|c| c == 'x')
                {
                    continue;
                }
                // Other words, not other spellings: American English has
                // |amongst| and |whilst| too, if less often.
                if matches!(
                    w.to_lowercase().as_str(),
                    "amongst" | "whilst" | "amidst" | "among" | "while" | "amid"
                ) {
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
    named_errors(s, false)
}

/// The errors of the best analyses of the grammar-error variant when they
/// disagree: each best analysis names one error, and no error is named by
/// all of them (|He will makes it|: |will makes| or |He will| for |His
/// will|). The sentence is proved ungrammatical; which correction the
/// writer meant is not. Empty when `grammar_errors` reports, or when the
/// analyses do not each name exactly one error.
pub fn grammar_error_alternatives(s: &crate::Sentence) -> Vec<GrammarError> {
    named_errors(s, true)
}

/// Most analyses that disagree on the error that are worth reporting.
const MAX_ALTERNATIVES: usize = 5;

/// The least share of the probability of the best analyses for which an
/// alternative correction is offered.
const MIN_SHARE: f64 = 0.05;

fn named_errors(s: &crate::Sentence, alternatives: bool) -> Vec<GrammarError> {
    let Some(p) = &s.mal_parse else {
        return Vec::new();
    };
    let original: Vec<char> = s.original.chars().collect();
    // Capitals that are not errors: at the start of the sentence, after a
    // colon or a line break (list labels, verse), and in runs of
    // capitalized words (titles such as "Your Majesty", names such as
    // "Common Expression Language").
    let licensed_capital = |from: usize, to: usize| {
        let before: String = original[..from.min(original.len())].iter().collect();
        let before = before.trim_end_matches([' ', '\t']);
        let prev_word = before
            .rsplit(|c: char| c.is_whitespace())
            .next()
            .unwrap_or("");
        let next_word = original[to.min(original.len())..]
            .iter()
            .find(|c| !c.is_whitespace());
        before.trim().is_empty()
            || before.ends_with(':')
            || before.ends_with('\n')
            || prev_word.chars().next().is_some_and(char::is_uppercase)
            || next_word.is_some_and(|c| c.is_uppercase())
    };
    // A capital where none is expected (not the `nocap` errors, a missing
    // capital) that is licensed is not an error.
    let not_error = |code: &str, from: usize, to: usize| {
        code.contains("cap") && !code.contains("nocap") && licensed_capital(from, to)
    };
    let items = |r: &emdysi_parse::Reading| -> Vec<(String, usize, usize)> {
        let mut out = Vec::new();
        for n in r.nodes.iter().filter(|n| !n.leaf) {
            if is_error_item(&n.name) {
                out.push((n.name.clone(), n.from, n.to));
            }
            // Two clauses run together with no punctuation between them
            // (|He is go home| as |He is. Go home.|): the strict grammar
            // joins clauses only with a semicolon or a dash, and the
            // grammar-error variant names only a comma splice.
            if n.name == "cl-cl_runon_c" {
                let joined = n
                    .children
                    .first()
                    .and_then(|&c| r.nodes.get(c))
                    .and_then(|left| left.to.checked_sub(1))
                    .and_then(|i| original.get(i))
                    .is_some_and(|c| c.is_alphanumeric());
                if joined {
                    out.push((n.name.clone(), n.from, n.to));
                }
            }
        }
        // A word's error is named by its entry when the error table has the
        // entry (|be_c_was_rbst|), else by its lexical type, which is where
        // the table describes most grammar-error entries
        // (|do1_neg_1_u_mal|, of type |va_dont_neg_pres_le_rbst|).
        let table = crate::dict::erg_errors();
        for w in &r.words {
            let code = if table.contains_key(&w.entry) {
                &w.entry
            } else if is_error_item(&w.le_type) {
                &w.le_type
            } else if is_error_item(&w.entry) {
                &w.entry
            } else {
                continue;
            };
            // A past tense after |has| or |have| is named either on the
            // auxiliary or as a wrong participle of the verb after it; both
            // are the error of the verb (|has went|, |has wrote|).
            if code == "has_aux_finc_rbst" || code == "have_aux_finc_rbst" {
                if let Some(next) = r
                    .words
                    .iter()
                    .filter(|x| x.from >= w.to)
                    .min_by_key(|x| x.from)
                {
                    out.push(("v_psp_olr_rbst".to_string(), next.from, next.to));
                    continue;
                }
            }
            out.push((code.clone(), w.from, w.to));
        }
        out.retain(|(code, from, to)| !not_error(code, *from, *to));
        out
    };
    // Whole-sentence analyses are preferred to fragments; among them, the
    // one that assumes the fewest errors.
    // A reading glued together from fragments (|He| as a name, |go| as a
    // command, |to school every day| as a verb-phrase fragment) is not a
    // whole-sentence analysis, whatever its root.
    let sentence_root = |r: &emdysi_parse::Reading| {
        (matches!(
            r.root.as_str(),
            "root_decl" | "root_question" | "root_command" | "root_robust_s" | "root_robust_ques"
        ) || emdysi_parse::is_strict(&r.root))
            && !r.nodes.iter().any(|n| !n.leaf && n.name.contains("frg"))
    };
    // An error is reported only when the grammar has proved that the
    // sentence is outside it (a complete search found no strict or
    // informal analysis), and only when every analysis of the
    // grammar-error variant that assumes the fewest errors names it: a
    // coverage gap or an ambiguity is not an error.
    if !s.strict() {
        let proved = s
            .parse
            .as_ref()
            .is_some_and(|q| q.complete && q.readings.iter().all(|r| r.root == "fragment"));
        if !proved {
            return Vec::new();
        }
        // The grammar can vouch for an error only among words it knows: an
        // analysis that leans on a generic entry for an unknown word (a
        // capitalized plural read as a name, a noun the lexicon has only
        // as a verb) says more about the lexicon than about the sentence.
        // (An error on the unknown word itself, such as "buyed", is let
        // through below.)
    }
    // Words no analysis knows: every analysis that covers them uses a
    // generic entry (a capitalized first word may have a generic reading
    // besides its known one; that does not count).
    let words: Vec<&emdysi_parse::Word> = s
        .parse
        .iter()
        .chain(std::iter::once(p))
        .flat_map(|q| q.readings.iter())
        .flat_map(|r| r.words.iter())
        .collect();
    let generic_spans: Vec<(usize, usize)> = words
        .iter()
        .filter(|w| w.generic)
        .map(|w| (w.from, w.to))
        .filter(|&(f, t)| !words.iter().any(|w| !w.generic && w.from == f && w.to == t))
        .collect();
    type Items = Vec<(String, usize, usize)>;
    let pick = |whole: bool| -> Option<(Items, Vec<(Items, f64)>)> {
        let sets: Vec<(Items, f64)> = p
            .readings
            .iter()
            .filter(|r| !whole || sentence_root(r))
            // A generic entry for a word the lexicon knows (|He| as a plural
            // name, so that |He go| agrees) is not an analysis of the
            // sentence.
            .filter(|r| {
                r.words
                    .iter()
                    .filter(|w| w.generic)
                    .all(|w| generic_spans.contains(&(w.from, w.to)))
            })
            .map(|r| (items(r), r.score))
            .collect();
        let fewest = sets.iter().map(|e| e.0.len()).min()?;
        let best: Vec<(Items, f64)> = sets.into_iter().filter(|e| e.0.len() == fewest).collect();
        let shared = best[0]
            .0
            .iter()
            .filter(|e| best[1..].iter().all(|o| o.0.contains(e)))
            .cloned()
            .collect();
        Some((shared, best))
    };
    let Some((shared, sets)) = pick(true).or_else(|| pick(false)) else {
        return Vec::new();
    };
    let best = if alternatives {
        if !shared.is_empty() || sets.iter().any(|e| e.0.len() != 1) {
            return Vec::new();
        }
        // Each error's share of the probability of the best analyses,
        // which orders the corrections; the ranker never decides whether
        // there is an error, which the strict grammar has proved.
        // The model's probabilities are proportional to
        // `exp(score / temperature)` (see `Parse::temperature`).
        let t = p.temperature;
        let top = sets.iter().map(|e| e.1).fold(f64::NEG_INFINITY, f64::max);
        let total: f64 = sets.iter().map(|e| ((e.1 - top) / t).exp()).sum();
        let mut alts: Vec<((String, usize, usize), f64)> = Vec::new();
        for (e, score) in sets {
            let share = ((score - top) / t).exp() / total;
            let e = e.into_iter().next().expect("one error");
            match alts.iter_mut().find(|a| a.0 == e) {
                Some(a) => a.1 += share,
                None => alts.push((e, share)),
            }
        }
        if alts.len() < 2 || alts.len() > MAX_ALTERNATIVES {
            return Vec::new();
        }
        alts.sort_by(|a, b| b.1.total_cmp(&a.1));
        // As for ambiguity, an analysis with less than 5% is not offered.
        alts.retain(|a| a.1 >= MIN_SHARE);
        alts.into_iter().map(|a| a.0).collect()
    } else {
        shared
    };
    let wanted = best.len();
    // A reading that needs many corrections is the grammar-error variant
    // making the best of a sentence the grammar could not analyse (long,
    // or with a construction it lacks), not a list of real errors.
    if !alternatives && best.len() > MAX_ERRORS {
        return Vec::new();
    }
    let chars = &original;
    let mut out: Vec<GrammarError> = Vec::new();
    for (code, from, to) in best {
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
    // Code masked for the parser (see `parser_text`) is read as a name.
    let masked: Vec<char> = crate::parser_text(&s.original, false)
        .unwrap_or_default()
        .chars()
        .collect();
    let placeholder = |f: usize, t: usize| crate::masked_code(&masked, f, t);
    let acronym = |f: usize, t: usize| crate::acronym(&original, f, t);
    if !s.strict()
        && generic_spans
            .iter()
            .filter(|&&(f, t)| !acronym(f, t) && !placeholder(f, t))
            .any(|&(f, t)| !out.iter().any(|e| e.from < t && f < e.to))
    {
        return Vec::new();
    }
    // When the grammar has a strict analysis that treats some word as
    // unknown (re-parsed for wrong forms such as "buyed"), only errors
    // within those words count, and not within an acronym or masked code
    // (see `crate::name_like`): elsewhere the strict analysis stands.
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
        out.retain(|e| {
            unknown
                .iter()
                .any(|&(f, t)| f <= e.from && e.to <= t && !acronym(f, t) && !placeholder(f, t))
        });
    }
    // An alternative that is not an error (a licensed capital) leaves an
    // analysis without one.
    if alternatives && out.len() != wanted {
        return Vec::new();
    }
    out
}

/// Pronouns and their other cases, for errors of pronoun case.
const PRONOUN_CASES: &[&[&str]] = &[
    &["i", "me", "my", "mine", "myself"],
    &["we", "us", "our", "ours", "ourselves"],
    &["you", "your", "yours", "yourself", "yourselves"],
    &["he", "him", "his", "himself"],
    &["she", "her", "hers", "herself"],
    &["it", "its", "itself"],
    &["they", "them", "their", "theirs", "themselves"],
    &["who", "whom", "whose"],
];

/// Corrections to try for a named error, from the kind of error its
/// feedback describes: (start, end, replacement) in characters of the
/// sentence. Errors of other kinds get none.
/// Whether `a` and `b` are the two numbers of one present- or past-tense
/// verb form: |borrow| and |borrows|, |try| and |tries|, |has| and |have|,
/// |was| and |were|.
/// Singular and plural forms of the verbs whose forms are separate words
/// in the grammar, not inflections of one stem.
const IRREGULAR_NUMBER: &[(&str, &str)] = &[
    ("is", "are"),
    ("am", "are"),
    ("was", "were"),
    ("has", "have"),
    ("does", "do"),
    ("doesn't", "don't"),
    ("isn't", "aren't"),
    ("wasn't", "weren't"),
    ("hasn't", "haven't"),
];

/// The other number of an irregular verb form (|was| and |were|).
fn irregular_number(w: &str) -> Vec<&'static str> {
    IRREGULAR_NUMBER
        .iter()
        .filter_map(|&(x, y)| {
            if w == x {
                Some(y)
            } else if w == y {
                Some(x)
            } else {
                None
            }
        })
        .collect()
}

fn number_pair(a: &str, b: &str) -> bool {
    const IRREGULAR: &[(&str, &str)] = IRREGULAR_NUMBER;
    let third = |base: &str, s: &str| {
        s == format!("{base}s")
            || s == format!("{base}es")
            || base
                .strip_suffix('y')
                .is_some_and(|stem| s == format!("{stem}ies"))
    };
    let irregular = |w: &str| IRREGULAR.iter().any(|&(x, y)| w == x || w == y);
    if irregular(a) || irregular(b) {
        // |was| is not |wa| plus -s.
        return IRREGULAR
            .iter()
            .any(|&(x, y)| (a, b) == (x, y) || (a, b) == (y, x));
    }
    third(a, b) || third(b, a)
}

fn candidate_fixes(
    erg: &Erg,
    s: &crate::Sentence,
    e: &GrammarError,
    names: &std::collections::HashSet<String>,
) -> Vec<(usize, usize, String)> {
    let f = e.feedback.to_lowercase();
    let word = e.text.as_str();
    let lower = word.to_lowercase();
    let (from, to) = (e.from, e.to);
    let chars: Vec<char> = s.original.chars().collect();
    // The word before the error, for articles.
    let before: String = chars[..from.min(chars.len())].iter().collect();
    let prev = before.trim_end();
    let prev_word_start = prev
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_whitespace())
        .map_or(0, |(i, _)| prev[..i].chars().count() + 1);
    let prev_word: String = prev.chars().skip(prev_word_start).collect();
    let mut out: Vec<(usize, usize, String)> = Vec::new();
    let has = |k: &[&str]| k.iter().any(|x| f.contains(x));
    // Two clauses run together at a subject pronoun: a semicolon before it
    // when the pronoun starts a second clause (|We fixed it; we shipped
    // it|), else the pronoun deleted with the space before it, a stray one
    // (|let's we dive|, |enable this feature it by setting|). Only where
    // the clauses meet without punctuation, and by the word before.
    if e.code == "cl-cl_runon_c" {
        let mut i = from;
        while i < to.min(chars.len()) {
            let start = i;
            while i < to && chars[i].is_alphabetic() {
                i += 1;
            }
            let w: String = chars[start..i].iter().collect::<String>().to_lowercase();
            if start > from + 1
                && chars[start - 1] == ' '
                && chars[start - 2].is_alphanumeric()
                && ["i", "we", "you", "he", "she", "it", "they"].contains(&w.as_str())
            {
                // The word before, in the likeliest analysis: a clause can
                // end in a noun, adjective or adverb (|fixed it; we|), not
                // in a verb, which a semicolon would leave elliptical
                // (|let's; we dive|), nor in a conjunction or preposition
                // (|easier to read if; you|). A stray pronoun follows a
                // noun or a verb.
                let before = s
                    .mal_parse
                    .iter()
                    .flat_map(|m| m.readings.first())
                    .flat_map(|r| r.words.iter())
                    .find(|w| w.to + 1 == start)
                    .map(|w| w.le_type.as_str())
                    .unwrap_or("");
                let class = |p: &[&str]| p.iter().any(|x| before.starts_with(x));
                if class(&["n_", "aj_", "av_"]) {
                    let pronoun: String = chars[start..i].iter().collect();
                    out.push((start - 1, i, format!("; {pronoun}")));
                }
                if class(&["n_", "v_"]) {
                    out.push((start - 1, i, String::new()));
                }
            }
            i += 1;
        }
        return out;
    }
    if word.contains(char::is_whitespace) {
        return out;
    }
    // A capitalized word after the start of the sentence is a name
    // ("Rust", "Pod"), and a number is not a word to inflect: the grammar
    // knows too little about either to name an error in them.
    let pronoun = PRONOUN_CASES
        .iter()
        .any(|set| set.contains(&lower.as_str()));
    let sentence_start = prev
        .trim_start_matches(|c: char| !c.is_alphanumeric())
        .is_empty();
    let name_like = word.chars().next().is_some_and(char::is_uppercase)
        && (!sentence_start || names.contains(word));
    if !pronoun && (name_like || word.chars().any(|c| c.is_ascii_digit())) {
        return out;
    }
    if has(&["doubled word"]) {
        // Delete the word and the space before it.
        out.push((
            prev_word_start + prev_word.chars().count(),
            to,
            String::new(),
        ));
    }
    if has(&[
        "agree",
        "irregular",
        "form of the verb",
        "wrong form",
        "plural form",
        "singular form",
        "correct form",
    ]) {
        // The forms closest to what was written first: "running" before
        // "runes" for "runing".
        let mut forms = erg.inflections(word);
        // |was| and |were| are separate words in the grammar, not forms of
        // one stem.
        for other in irregular_number(&lower) {
            if !forms.iter().any(|f| f == other) {
                forms.push(other.to_string());
            }
        }
        forms.sort_by_key(|f| (edit_distance(word, f), crate::dict::tier(f).is_none()));
        // Agreement is between a subject and a finite verb: only finite
        // forms repair it ("aspiring" would not repair "to aspire" but
        // build another phrase), and the first word of a sentence has no
        // subject before it ("Ensure that ...", "Try ...").
        // And it is repaired by the other number of the same tense (|borrow|
        // and |borrows|, |has| and |have|), not by another tense (|borrowed|
        // and |had| parse, but say something else).
        let agreement = f.contains("agree");
        // A regular past of a verb with an irregular one is repaired by the
        // irregular form (|buyed| by |bought|), not by another tense (|buy|).
        if f.contains("past participle") {
            forms = erg.irregular_participles(word);
        } else if f.contains("irregular") && f.contains("past") {
            forms = erg.irregular_pasts(word);
        }
        if !(agreement && sentence_start) {
            for form in forms {
                if agreement && !number_pair(&lower, &form) {
                    continue;
                }
                out.push((from, to, match_case(word, &form)));
            }
        }
    }
    // An auxiliary followed by the wrong form of a verb: the repair is on
    // the verb, its base form after a modal (|can goes|) or its -ing form
    // after |be| (|is go|).
    let next_verb = |rule: Option<&str>| -> Vec<(usize, usize, String)> {
        let after: String = chars[to.min(chars.len())..].iter().collect();
        let start = to + after.chars().take_while(|c| c.is_whitespace()).count();
        let next: String = chars[start.min(chars.len())..]
            .iter()
            .take_while(|c| c.is_alphabetic())
            .collect();
        if next.is_empty() {
            return Vec::new();
        }
        erg.forms_by_rule(&next, rule)
            .into_iter()
            .filter(|f| crate::dict::tier(f).is_some())
            .map(|f| (start, start + next.chars().count(), match_case(&next, &f)))
            .collect()
    };
    // A mass noun made plural (|informations|): its singular, and, when a
    // verb agrees with the plural (|informations are|), the singular with
    // that verb in the singular too (|information is|).
    // Only when the word list has no such plural: |lints|, |coercions|,
    // |researches| are count uses the grammar's lexicon lacks.
    if has(&["always singular"])
        && crate::dict::tier(&lower).is_none()
        && !crate::dict::accepted_common(&lower)
    {
        for base in erg
            .forms_by_rule(word, None)
            .into_iter()
            .filter(|f| crate::dict::tier(f).is_some())
        {
            let base = match_case(word, &base);
            out.push((from, to, base.clone()));
            // The first auxiliary in the next few words with a form of the
            // other number (|are|, |were|, |have|, |do|) is the verb that
            // agrees.
            let mut i = to;
            for k in 0..6 {
                while i < chars.len() && !chars[i].is_alphabetic() {
                    i += 1;
                }
                let start = i;
                while i < chars.len() && (chars[i].is_alphabetic() || chars[i] == '\'') {
                    i += 1;
                }
                if start == i {
                    break;
                }
                let w: String = chars[start..i].iter().collect();
                let lw = w.to_lowercase();
                let mut others: Vec<String> = irregular_number(&lw)
                    .iter()
                    .map(|o| o.to_string())
                    .collect();
                // A verb right after the noun takes its singular form
                // (|informations help| as |information helps|).
                if others.is_empty() && k == 0 {
                    others = erg
                        .forms_by_rule(&lw, Some("v_3s-fin_olr"))
                        .into_iter()
                        .filter(|f| crate::dict::tier(f).is_some())
                        // Only from the base form: not |given|.
                        .filter(|f| erg.forms_by_rule(f, None).contains(&lw))
                        .collect();
                }
                if !others.is_empty() {
                    let between: String = chars[to..start].iter().collect();
                    for o in others {
                        out.push((from, i, format!("{base}{between}{}", match_case(&w, &o))));
                    }
                    break;
                }
            }
        }
    }
    if has(&["should not be inflected", "should be the base form"]) {
        out.extend(next_verb(None));
    }
    if has(&["present participle"]) {
        out.extend(next_verb(Some("v_prp_olr")));
    }
    // |has went|: the error is on the auxiliary, the repair on the verb
    // after it, which takes its past participle (|has gone|).
    if has(&["participle form"]) {
        let after: String = chars[to.min(chars.len())..].iter().collect();
        let start = to + after.chars().take_while(|c| c.is_whitespace()).count();
        let next: String = chars[start.min(chars.len())..]
            .iter()
            .take_while(|c| c.is_alphabetic())
            .collect();
        if !next.is_empty() {
            for p in erg.irregular_participles(&next) {
                out.push((start, start + next.chars().count(), match_case(&next, &p)));
            }
        }
    }
    let pl = prev_word.to_lowercase();
    // A missing article is not claimed: a bare noun is often right in
    // edited text (a mass use, "hopeless of remedy"; headline style, "Goal
    // is to ..."), and the grammar cannot tell these from an error.
    if has(&["“a”", "\"a\"", "“an”", "\"an\"", "article"]) {
        let pw_end = prev_word_start + prev_word.chars().count();
        match pl.as_str() {
            "a" => out.push((prev_word_start, pw_end, match_case(&prev_word, "an"))),
            "an" => out.push((prev_word_start, pw_end, match_case(&prev_word, "a"))),
            _ => {}
        }
        if ["a", "an", "the"].contains(&pl.as_str()) {
            // Without the article: delete it and the space after it.
            out.push((prev_word_start, from, String::new()));
        }
        if ["a", "an"].contains(&lower.as_str()) {
            let other = if lower == "a" { "an" } else { "a" };
            out.push((from, to, match_case(word, other)));
        }
    }
    if has(&["pronoun", "“am”", "\"am\""]) {
        // Subject and object forms only (he/him): a possessive in place of
        // a pronoun builds another phrase rather than repairing one.
        for (subject, object) in [
            ("i", "me"),
            ("we", "us"),
            ("he", "him"),
            ("she", "her"),
            ("they", "them"),
            ("who", "whom"),
        ] {
            if lower == subject {
                out.push((from, to, match_case(word, object)));
            } else if lower == object {
                out.push((from, to, match_case(word, subject)));
            }
        }
        for v in ["am", "is", "are"] {
            if lower != v && ["am", "is", "are"].contains(&lower.as_str()) {
                out.push((from, to, match_case(word, v)));
            }
        }
    }
    for (a, b) in [
        ("fewer", "less"),
        ("less", "fewer"),
        ("if", "whether"),
        ("after", "afterward"),
    ] {
        if lower == a && f.contains(b) {
            out.push((from, to, match_case(word, b)));
        }
    }
    out
}

/// A correction of a named error that the strict grammar accepts: the
/// error is real only if fixing it the way its kind suggests turns the
/// sentence into one the grammar analyses strictly. A sentence that is
/// correct English outside the grammar's coverage stays unanalysable
/// after such a small change, so it is not reported. Returns the span it
/// replaces (the error's, widened to the words the correction changes,
/// such as the verb in |informations are|) and the replacement.
fn verified_fix(
    erg: &Erg,
    s: &crate::Sentence,
    e: &GrammarError,
    names: &std::collections::HashSet<String>,
) -> Option<(usize, usize, String)> {
    let chars: Vec<char> = s.original.chars().collect();
    // The corrected sentence is parsed as the sentence was, with code
    // masked (see `parser_text`); the positions are the same.
    let masked: Vec<char> = crate::parser_text(&s.original, false)
        .map(|m| m.chars().collect())
        .unwrap_or_else(|| chars.clone());
    for (from, to, rep) in candidate_fixes(erg, s, e, names) {
        if from > to || to > chars.len() || masked.len() != chars.len() {
            continue;
        }
        let fixed: String = masked[..from]
            .iter()
            .chain(rep.chars().collect::<Vec<_>>().iter())
            .chain(masked[to..].iter())
            .collect();
        let Ok(p) = crate::timing::timed(&crate::timing::VERIFY, || {
            erg.parse_limited(
                fixed.trim(),
                std::time::Duration::from_secs(5),
                VERIFY_READINGS,
            )
        }) else {
            continue;
        };
        // The correction must repair the analysis that names the error, not
        // make way for another one: some strict reading of the corrected
        // sentence keeps the lexical entry of every other word (|much good
        // may it does them| parses only with |may| as a noun). A word the
        // erroneous analysis covers with a generic entry (|Binding| as an
        // unknown name) may have any.
        let lead = fixed.chars().count() - fixed.trim_start().chars().count();
        let edited = (from.min(e.from), to.max(e.to));
        let delta = rep.chars().count() as isize - (to - from) as isize;
        let edited_fixed = (
            edited.0,
            (edited.1 as isize + delta).max(edited.0 as isize) as usize,
        );
        let others = |words: &[emdysi_parse::Word], skip: (usize, usize), shift: usize| {
            words
                .iter()
                .filter(|w| !(w.from + shift < skip.1 && w.to + shift > skip.0))
                .map(|w| (!w.generic).then(|| w.entry.clone()))
                .collect::<Vec<Option<String>>>()
        };
        let keeps = |wrong: &[Option<String>], fixed: &[Option<String>]| {
            wrong.len() == fixed.len()
                && wrong.iter().zip(fixed).all(|(a, b)| a.is_none() || a == b)
        };
        let erroneous: Vec<Vec<Option<String>>> = s
            .mal_parse
            .iter()
            .flat_map(|m| m.readings.iter())
            .filter(|r| {
                r.nodes
                    .iter()
                    .any(|n| n.name == e.code && n.from == e.from && n.to == e.to)
                    || r.words
                        .iter()
                        .any(|w| w.le_type == e.code && w.from == e.from && w.to == e.to)
            })
            .map(|r| others(&r.words, edited, 0))
            .collect();
        if p.readings.iter().any(|r| {
            emdysi_parse::is_strict(&r.root)
                && !crate::unlicensed_quoting(r, fixed.trim())
                && (erroneous.is_empty() || {
                    let fixed = others(&r.words, edited_fixed, lead);
                    erroneous.iter().any(|w| keeps(w, &fixed))
                })
        }) {
            // A run-on's repair is at one place in the clause it spans.
            if e.code == "cl-cl_runon_c" {
                return Some((from, to, rep));
            }
            let new: String = chars[from.min(e.from)..from]
                .iter()
                .chain(rep.chars().collect::<Vec<_>>().iter())
                .chain(chars[to..to.max(e.to)].iter())
                .collect();
            return Some((from.min(e.from), to.max(e.to), new));
        }
    }
    None
}

/// Readings of a corrected sentence searched for one that keeps the
/// other words' entries: the one that does is not always among the best
/// few.
const VERIFY_READINGS: usize = 100;

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
/// Whether `b` is as few edits from `a` with insertions and deletions
/// only as with substitutions too.
fn indels_only(a: &str, b: &str) -> bool {
    let (x, y): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    // Longest common subsequence.
    let mut row = vec![0usize; y.len() + 1];
    for &cx in &x {
        let mut prev = 0;
        for (j, &cy) in y.iter().enumerate() {
            let here = row[j + 1];
            row[j + 1] = if cx == cy {
                prev + 1
            } else {
                row[j + 1].max(row[j])
            };
            prev = here;
        }
    }
    x.len() + y.len() - 2 * row[y.len()] == edit_distance(a, b)
}

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
/// Shortest misspelling reported when the word meant is two edits away.
const TWO_EDIT_LENGTH: usize = 7;
/// Least common tier of the word meant, when it is two edits away.
const TWO_EDIT_TIER: u8 = 50;
/// The tier of the words a three-letter typo may be meant for.
const SHORT_TIER: u8 = 35;
/// At most this many common words two edits away: the model chooses
/// among them (|furutre|: |furore|, |future|, |nurture|).
const TWO_EDIT_CANDIDATES: usize = 4;

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
    // Listed words first: the grammar's morphology also derives non-words
    // ("doabled" for "diabled").
    let listed = |w: &str| crate::dict::tier(w).is_some();
    found.sort_by(|x, y| {
        (!listed(&x.2))
            .cmp(&!listed(&y.2))
            .then(x.0.total_cmp(&y.0))
            .then(x.1.cmp(&y.1))
            .then(x.2.cmp(&y.2))
    });
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

/// Why the grammar-error check does or does not claim an error in a
/// sentence, step by step: for working on the checks (the `why`
/// example).
pub fn explain_grammar(erg: &Erg, a: &crate::Analysis, si: usize) -> String {
    let s = &a.sentences[si];
    let names: std::collections::HashSet<String> = a
        .sentences
        .iter()
        .flat_map(|s| s.tokens.iter().skip(1))
        .filter(|t| t.form.chars().next().is_some_and(char::is_uppercase))
        .map(|t| t.form.clone())
        .collect();
    let mut out = Vec::new();
    match &s.parse {
        None => out.push(format!(
            "not parsed: {}",
            s.skipped.as_deref().unwrap_or("?")
        )),
        Some(p) => {
            let strict = p
                .readings
                .iter()
                .filter(|r| emdysi_parse::is_strict(&r.root))
                .count();
            out.push(format!(
                "parse: {} readings, {strict} strict, complete={}, exhausted={}",
                p.readings.len(),
                p.complete,
                p.exhausted
            ));
            if let Some(r) = p.readings.first() {
                out.push(format!("  best: [{}] {}", r.root, r.derivation));
            }
        }
    }
    match &s.mal_parse {
        None => out.push("grammar-error variant: not asked".into()),
        Some(m) => {
            out.push(format!(
                "grammar-error variant: {} readings",
                m.readings.len()
            ));
            for r in m.readings.iter().take(5) {
                let mut errs: Vec<String> = r
                    .nodes
                    .iter()
                    .filter(|n| is_error_item(&n.name))
                    .map(|n| format!("{}@{}-{}", n.name, n.from, n.to))
                    .collect();
                errs.extend(
                    r.words
                        .iter()
                        .filter(|w| w.generic)
                        .map(|w| format!("generic:{}@{}-{}", w.surface, w.from, w.to)),
                );
                out.push(format!("  [{}] {:.1} {}", r.root, r.score, errs.join(" ")));
            }
        }
    }
    let named = grammar_errors(s);
    let alts = grammar_error_alternatives(s);
    if named.is_empty() && alts.is_empty() {
        out.push("named: none".into());
    }
    for (kind, errs) in [("named", &named), ("alternative", &alts)] {
        for e in errs.iter() {
            let fixes = candidate_fixes(erg, s, e, &names);
            let verified = verified_fix(erg, s, e, &names);
            out.push(format!(
                "{kind}: {} on '{}': candidates {:?}, verified {:?}",
                e.code,
                e.text,
                fixes.iter().map(|f| &f.2).collect::<Vec<_>>(),
                verified.map(|v| v.2)
            ));
        }
    }
    out.join("\n")
}

/// A word of a phrase in a rule, split as the tokenizer splits it: with a
/// curly apostrophe, a contraction as its own token (|here’s| is |here|
/// |’s|, |isn’t| is |is| |n’t|) and a hyphen as its own token, so that
/// |here's the thing| and |fast-paced| match.
fn split_contraction(w: &str) -> Vec<String> {
    // Trailing punctuation is a token of its own, as the tokenizer makes
    // it (|certainly!| is |certainly| |!|).
    if let Some(stem) = w
        .strip_suffix(['!', '?', '.', ',', ';', ':'])
        .filter(|s| !s.is_empty())
    {
        let mut parts = split_contraction(stem);
        parts.push(w[stem.len()..].to_string());
        return parts;
    }
    // |fast-paced| is |fast| |-| |paced|.
    if w.len() > 1 && w.contains('-') && !w.starts_with('-') && !w.ends_with('-') {
        let mut parts = Vec::new();
        for (i, p) in w.split('-').enumerate() {
            if i > 0 {
                parts.push("-".to_string());
            }
            parts.extend(split_contraction(p));
        }
        return parts;
    }
    let w = w.replace('\'', "’");
    if let Some(stem) = w.strip_suffix("n’t").filter(|s| !s.is_empty()) {
        return vec![stem.to_string(), "n’t".to_string()];
    }
    for suffix in ["’s", "’re", "’ve", "’ll", "’d", "’m"] {
        if let Some(stem) = w.strip_suffix(suffix).filter(|s| !s.is_empty()) {
            return vec![stem.to_string(), suffix.to_string()];
        }
    }
    vec![w]
}

/// Whether a match meets a part-of-speech condition: the words of the
/// sentence's best reading that the group covers.
fn tag_holds(s: &crate::Sentence, m: &fancy_regex::Captures<'_, str>, c: &TagCheck) -> bool {
    let Some(g) = m.name(&c.group) else {
        // An optional group that did not take part.
        return true;
    };
    let from = s.original[..g.start()].chars().count();
    let to = from + g.as_str().chars().count();
    let found = s.best().is_some_and(|r| {
        r.words
            .iter()
            .filter(|w| w.from < to && from < w.to)
            .flat_map(crate::tags::penn_all)
            .any(|t| c.tag.is_match(t).unwrap_or(false))
    });
    found != c.negate
}

/// Whether bytes `[from, to)` of `text` touch a URL or a file path: a
/// run of non-space characters with |://|, a slash between letters
/// (|src/main|, not |2/29|), or a dot between a letter or digit and two
/// or more letters, the first lower case (|github.io|, |main.rs|, not
/// |a.m.|, |e.g.| or |B.Sc|).
fn in_url_or_path(text: &str, from: usize, to: usize) -> bool {
    let start = text[..from].rfind(char::is_whitespace).map_or(0, |i| i + 1);
    let end = text[to..]
        .find(char::is_whitespace)
        .map_or(text.len(), |i| to + i);
    let c: Vec<char> = text[start..end].chars().collect();
    let letter = |i: usize| c.get(i).is_some_and(|c| c.is_alphabetic());
    text[start..end].contains("://")
        || (1..c.len()).any(|i| {
            (c[i] == '/' && letter(i - 1) && letter(i + 1))
                || (c[i] == '.'
                    && c[i - 1].is_alphanumeric()
                    && letter(i + 1)
                    && letter(i + 2)
                    && c[i + 1].is_lowercase())
        })
}

/// The words inside a Markdown document's code: inline code spans and
/// fenced blocks, lowercased.
fn code_words(source: &str) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    let mut fenced = false;
    let mut add = |code: &str| {
        out.extend(
            code.split(|c: char| !c.is_alphanumeric())
                .filter(|w| !w.is_empty())
                .map(str::to_lowercase),
        )
    };
    for line in source.lines() {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            fenced = !fenced;
        } else if fenced {
            add(line);
        } else {
            for code in line.split('`').skip(1).step_by(2) {
                add(code);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::number_pair;

    #[test]
    fn number_pairs() {
        assert!(number_pair("go", "goes"));
        assert!(number_pair("tries", "try"));
        assert!(number_pair("was", "were"));
        assert!(!number_pair("was", "wa"));
        assert!(!number_pair("borrow", "borrowed"));
        assert!(!number_pair("has", "had"));
    }

    #[test]
    fn irregular_numbers() {
        assert_eq!(super::irregular_number("was"), ["were"]);
        assert_eq!(super::irregular_number("are"), ["is", "am"]);
        assert!(super::irregular_number("went").is_empty());
    }
}
