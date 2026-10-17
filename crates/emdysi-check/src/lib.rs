//! Checks over parsed English text: spelling, grammaticality, style and
//! substance rules from rule packs, with source positions and fixes.
//!
//! [`analyze`] splits a document into prose blocks and sentences and parses
//! each sentence with the English Resource Grammar. A [`Checker`] then runs
//! rule packs (see [`rules`]) over the analysis, producing [`Diagnostic`]s
//! that point into the original source and may carry a replacement.

use std::ops::Range;
use std::sync::Mutex;
use std::time::Duration;

use emdysi_parse::{Erg, InputToken, Parse, Reading};
use emdysi_text::blocks::{Block, BlockKind, markdown_blocks, plain_blocks};
use emdysi_text::segment::sentences;

/// Time spent in each stage of checking, summed over all threads since the
/// process started (for profiling: see `examples/timing.rs`).
pub mod timing {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, Instant};

    pub static PARSE: AtomicU64 = AtomicU64::new(0);
    pub static REPARSE: AtomicU64 = AtomicU64::new(0);
    pub static MAL: AtomicU64 = AtomicU64::new(0);
    pub static RULES: AtomicU64 = AtomicU64::new(0);
    pub static VERIFY: AtomicU64 = AtomicU64::new(0);

    /// Runs `f`, adding its time to `counter`.
    pub fn timed<T>(counter: &AtomicU64, f: impl FnOnce() -> T) -> T {
        let t = Instant::now();
        let out = f();
        counter.fetch_add(t.elapsed().as_nanos() as u64, Ordering::Relaxed);
        out
    }

    /// Time per rule id, summed like the stages.
    pub static RULE_TIMES: std::sync::Mutex<std::collections::BTreeMap<String, Duration>> =
        std::sync::Mutex::new(std::collections::BTreeMap::new());

    /// The stages and their times so far.
    pub fn report() -> Vec<(&'static str, Duration)> {
        [
            ("parse", &PARSE),
            ("re-parse (quoting, unpruned)", &REPARSE),
            ("grammar-error variant", &MAL),
            ("rules", &RULES),
            ("rules' correction parses", &VERIFY),
        ]
        .into_iter()
        .map(|(n, c)| (n, Duration::from_nanos(c.load(Ordering::Relaxed))))
        .collect()
    }
}

pub mod articles;
pub mod assist;
pub mod compounds;
pub mod decisions;
pub mod dict;
pub mod glossary;
pub mod modifiers;
pub mod repeats;
pub mod report;
pub mod rules;
pub mod semantics;
pub mod structure;
pub mod terms;
pub mod toml;

pub use rules::{Pack, Rule};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Plain,
    Markdown,
}

impl Format {
    /// Guess from a file name: `.md`/`.markdown` are Markdown.
    pub fn from_path(p: &std::path::Path) -> Format {
        match p.extension().and_then(|e| e.to_str()) {
            Some("md" | "markdown" | "mdx") => Format::Markdown,
            _ => Format::Plain,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Sentence {
    pub block: usize,
    /// Byte range in the block's text.
    pub range: Range<usize>,
    /// The sentence as written.
    pub original: String,
    /// The text given to the parser: inline code masked as `x`.
    pub text: String,
    pub parse: Option<Parse>,
    /// Tokens, from the parse or from tokenization alone.
    pub tokens: Vec<InputToken>,
    /// Why the sentence was not parsed, if it was not.
    pub skipped: Option<String>,
    /// Analysis by the grammar-error variant of the grammar, for sentences
    /// without a strict analysis (see [`Options::diagnose`]).
    pub mal_parse: Option<Parse>,
}

impl Sentence {
    /// The best-ranked reading, if any.
    pub fn best(&self) -> Option<&Reading> {
        self.parse.as_ref().and_then(|p| p.readings.first())
    }

    /// Whether some reading uses the strict (non-fragment, non-informal)
    /// root condition.
    pub fn strict(&self) -> bool {
        self.parse
            .as_ref()
            .is_some_and(|p| p.readings.iter().any(|r| emdysi_parse::is_strict(&r.root)))
    }

    pub fn word_count(&self) -> usize {
        self.tokens
            .iter()
            .filter(|t| t.form.chars().any(char::is_alphanumeric))
            .count()
    }
}

#[derive(Debug, Clone)]
pub struct Analysis {
    pub source: String,
    pub format: Format,
    pub blocks: Vec<Block>,
    pub sentences: Vec<Sentence>,
}

impl Analysis {
    /// Source byte range for characters `[from, to)` of sentence `s`.
    pub fn source_range(&self, s: usize, from: usize, to: usize) -> Range<usize> {
        let sent = &self.sentences[s];
        let block = &self.blocks[sent.block];
        let byte = |c: usize| -> usize {
            sent.original
                .char_indices()
                .nth(c)
                .map_or(sent.original.len(), |(b, _)| b)
        };
        let (a, b) = (sent.range.start + byte(from), sent.range.start + byte(to));
        block.source_range(a..b)
    }

    /// Source byte range of a whole sentence.
    pub fn sentence_source(&self, s: usize) -> Range<usize> {
        let sent = &self.sentences[s];
        self.blocks[sent.block].source_range(sent.range.clone())
    }

    /// 1-based line and column of a source byte offset.
    pub fn line_col(&self, byte: usize) -> (usize, usize) {
        let before = &self.source[..byte.min(self.source.len())];
        let line = before.matches('\n').count() + 1;
        let col = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
        (line, col)
    }
}

#[derive(Debug, Clone)]
pub struct Options {
    /// Run the parser (otherwise only tokenization).
    pub parse: bool,
    pub threads: usize,
    /// Sentences with more tokens than this are not parsed.
    pub max_tokens: usize,
    pub timeout: Duration,
    /// Readings recovered per sentence; rules look at the best ones.
    pub max_readings: usize,
    /// Re-parse sentences without a strict analysis with the grammar-error
    /// variant of the grammar, to name the error.
    pub diagnose: bool,
    /// With `diagnose`, re-parse every sentence without a strict analysis,
    /// for its meaning (`en rewrite`) or to show it (`en parse`); otherwise
    /// only those whose errors can be reported: a complete search found
    /// fragments only (see `rules::grammar_errors`).
    pub meaning: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            parse: true,
            threads: std::thread::available_parallelism().map_or(2, |n| n.get()),
            max_tokens: 100,
            timeout: Duration::from_secs(10),
            max_readings: 100,
            diagnose: true,
            meaning: false,
        }
    }
}

/// Time limit and readings for the grammar-error variant of the grammar.
const MAL_TIMEOUT: Duration = Duration::from_secs(3);
const MAL_READINGS: usize = 20;

/// Readings recovered to make sure none is left out: a parse with fewer
/// has all of them.
const ALL_READINGS: usize = 1000;

/// The ERG's lexical rules for verbs of saying in quotations: inversion
/// (|He left, said Kim|) and a fragment for the quoted clause (|Yes, said
/// Kim|).
const QUOTING_RULES: &[&str] = &["v_inv-quot_dlr", "v_cp-frag_dlr"];

/// Whether a reading uses a verb of saying in a quotation where nothing is
/// quoted: no punctuation next to the verb (|Policy objects describes the
/// logic| as |Policy objects, "describes the logic"|, |the commands adds
/// two contexts| as |adds two, the commands|).
pub fn unlicensed_quoting(r: &Reading, text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    r.words.iter().any(|w| {
        w.rules.iter().any(|x| QUOTING_RULES.contains(&x.as_str())) && {
            let before = chars[..w.from.min(chars.len())]
                .iter()
                .rev()
                .find(|c| !c.is_whitespace());
            let after = chars[w.to.min(chars.len())..]
                .iter()
                .find(|c| !c.is_whitespace());
            let set_off = |c: Option<&char>| c.is_some_and(|c| !c.is_alphanumeric());
            !set_off(before) && !set_off(after)
        }
    })
}

/// The text of a list item's sentence that starts with a short label ("Set
/// goals: The terms ..."), with the label and its colon replaced by spaces, for the
/// parser: the label is not part of the clause and only slows the parse
/// down. Character positions are unchanged. `None` if there is no label.
fn mask_label(text: &str) -> Option<String> {
    let i = text.find(": ")?;
    let label = &text[..i];
    let rest = &text[i + 1..];
    if label.split_whitespace().count() > 5
        || label.contains(['.', '!', '?', '"', '“'])
        || rest.split_whitespace().count() < 3
    {
        return None;
    }
    Some(" ".repeat(label.chars().count() + 1) + rest)
}

/// Citations: author and year in parentheses ("(Walker, 2017)", "(see
/// Smith and Jones 2020, p. 4)") or numbers in brackets ("[3]", "[1, 4-6]").
static CITATION: std::sync::LazyLock<fancy_regex::Regex> = std::sync::LazyLock::new(|| {
    fancy_regex::Regex::new(
        r"\((?:see |e\.g\.,? |cf\. )?[A-Z][^()]{0,80}?\b(?:1[6-9]|20)\d\d[a-z]?(?:,\s*pp?\.\s*[\d–-]+)?\)|\[\d+(?:\s*[,–-]\s*\d+)*\]",
    )
    .expect("citation pattern")
});

/// Code-like tokens in prose: template shortcodes (`{{< glossary_tooltip
/// term_id="pod" >}}`, `{{% note %}}`, `{{ .Site.Title }}`), URLs, email
/// addresses, paths (`src/main.rs`) and names with a file or domain
/// extension (`cmd.exe`, `crates.io`), and command-line options
/// (`-funsigned-char`, `--dry-run`).
static CODE_LIKE: std::sync::LazyLock<fancy_regex::Regex> = std::sync::LazyLock::new(|| {
    fancy_regex::Regex::new(
        r"\{\{[<%]?[^{}]*?[>%]?\}\}|\bhttps?://[^\s<>()]+[^\s<>().,;:!?]|\b[\w.+-]+@[\w-]+(?:\.[\w-]+)+|(?<![\w/])(?:[\w.-]+/)+[\w-]*\.[A-Za-z0-9]+\b|(?<![\w/])/[\w.-]+(?:/[\w.-]+)+|\b[A-Za-z][\w-]*\.(?:io|com|org|net|dev|gov|edu|rs|exe|py|js|ts|md|toml|json|ya?ml|txt|html?|sh|go|c|h)\b|(?<![\w-])--?[a-z][a-z0-9]*(?:-[a-z0-9]+)+\b|(?<![\w-])--[a-z][a-z0-9]+\b",
    )
    .expect("code-like pattern")
});

/// The text the parser sees: citations, and labels of list items (see
/// [`mask_label`]), replaced by spaces; code-like tokens ([`CODE_LIKE`])
/// replaced by a capitalized placeholder of the same length, which the
/// grammar reads as a name. Citations are not part of the
/// sentence's grammar, and the parser would otherwise build them into a
/// noun phrase. Character positions are unchanged. `None` if nothing is
/// masked.
/// Whether characters `f..t` of a sentence masked for the parser (see
/// [`parser_text`]) are masked code, which the parser reads as a name.
pub(crate) fn masked_code(masked: &[char], f: usize, t: usize) -> bool {
    masked.get(f) == Some(&'X')
        && masked
            .get(f + 1..t)
            .is_some_and(|m| m.iter().all(|&c| c == 'x'))
}

/// Whether characters `f..t` of a sentence are an acronym (|CEL|, |RBAC|)
/// or its plural (|PRs|, |APIs|): a name, whatever the grammar knows of it.
pub(crate) fn acronym(original: &[char], f: usize, t: usize) -> bool {
    let w: String = original
        .get(f..t)
        .map(|c| c.iter().collect())
        .unwrap_or_default();
    let w = w
        .strip_suffix('s')
        .filter(|s| s.chars().count() >= 2)
        .unwrap_or(&w);
    w.chars().count() >= 2
        && w.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

/// Whether a sentence ends in a colon right after a subordinator (|CRDs are a
/// good fit if:|): the clause it opens is the list or block that follows, so
/// the sentence alone is incomplete on purpose.
fn lead_in(text: &str) -> bool {
    let Some(rest) = text.trim_end().strip_suffix(':') else {
        return false;
    };
    let last = rest
        .rsplit(|c: char| !c.is_alphabetic())
        .next()
        .unwrap_or("");
    matches!(
        last.to_lowercase().as_str(),
        "if" | "when"
            | "whenever"
            | "whether"
            | "because"
            | "unless"
            | "while"
            | "where"
            | "although"
            | "though"
            | "until"
    )
}

/// Whether characters `from..to` of a sentence lie inside a code-like token
/// ([`CODE_LIKE`]).
pub(crate) fn in_code_like(text: &str, from: usize, to: usize) -> bool {
    CODE_LIKE.find_iter(text).flatten().any(|m| {
        let start = text[..m.start()].chars().count();
        let end = start + m.as_str().chars().count();
        start <= from && to <= end
    })
}

pub(crate) fn parser_text(text: &str, in_list: bool) -> Option<String> {
    // A comma after a sentence-initial coordinating conjunction (|And, I
    // have a deep passion for this|, |Or, why did he come?|) is dropped for
    // the parser: the grammar reads a conjunction with a comma as informal,
    // and edited prose writes it. A space keeps the offsets.
    let unpunctuated = ["And, ", "But, ", "Or, ", "So, ", "Yet, "]
        .iter()
        .find(|c| text.starts_with(*c))
        .map(|c| format!("{} {}", &c[..c.len() - 2], &text[c.len()..]));
    let text = unpunctuated.as_deref().unwrap_or(text);
    let labelled = mask_label(text)
        .filter(|_| in_list)
        .or(unpunctuated.clone());
    let base = labelled.as_deref().unwrap_or(text);
    let mut spans: Vec<(usize, usize, bool)> = CITATION
        .find_iter(base)
        .flatten()
        .map(|m| (m.start(), m.end(), false))
        .collect();
    for m in CODE_LIKE.find_iter(base).flatten() {
        if !spans.iter().any(|&(a, b, _)| m.start() < b && a < m.end()) {
            spans.push((m.start(), m.end(), true));
        }
    }
    if spans.is_empty() {
        return labelled;
    }
    spans.sort_unstable();
    let mut out = String::with_capacity(base.len());
    let mut last = 0;
    for (start, end, code) in spans {
        out.push_str(&base[last..start]);
        let n = base[start..end].chars().count();
        if code {
            out.push('X');
            out.extend(std::iter::repeat_n('x', n - 1));
        } else {
            out.extend(std::iter::repeat_n(' ', n));
        }
        last = end;
    }
    out.push_str(&base[last..]);
    Some(out)
}

/// A sentence's parse outcome: index, parse, tokens, reason for skipping.
type Parsed = (
    usize,
    Option<Parse>,
    Vec<InputToken>,
    Option<String>,
    Option<Parse>,
);

/// Split a document into sentences and parse them.
pub fn analyze(erg: &Erg, source: &str, format: Format, opts: &Options) -> Analysis {
    let blocks = match format {
        Format::Markdown => markdown_blocks(source),
        Format::Plain => plain_blocks(source),
    };
    let mut sents = Vec::new();
    for (bi, b) in blocks.iter().enumerate() {
        // Headings and table cells are fragments by nature: one unit each.
        let ranges: Vec<Range<usize>> = match b.kind {
            BlockKind::Code => Vec::new(),
            BlockKind::Heading(_) | BlockKind::TableCell => {
                std::iter::once(0..b.text.len()).collect()
            }
            _ => sentences(&b.text),
        };
        for r in ranges {
            let original = b.text[r.clone()].to_string();
            let mut text = String::new();
            for (i, c) in original.char_indices() {
                let at = r.start + i;
                text.push(if b.is_opaque(&(at..at + 1)) && !c.is_whitespace() {
                    'x'
                } else {
                    c
                });
            }
            sents.push(Sentence {
                block: bi,
                range: r,
                original,
                text,
                parse: None,
                tokens: Vec::new(),
                skipped: None,
                mal_parse: None,
            });
        }
    }

    // Parse in parallel, longest sentences first for better load balance.
    let mut order: Vec<usize> = (0..sents.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(sents[i].text.len()));
    let results: Mutex<Vec<Parsed>> = Mutex::new(Vec::new());
    let next = Mutex::new(0usize);
    std::thread::scope(|s| {
        for _ in 0..opts.threads.max(1) {
            s.spawn(|| {
                loop {
                    let k = {
                        let mut n = next.lock().unwrap();
                        *n += 1;
                        *n - 1
                    };
                    let Some(&i) = order.get(k) else { break };
                    let text = &sents[i].text;
                    // Labels ("Keep going: ...") are a list-item convention;
                    // elsewhere a colon usually follows a clause ("There
                    // are three new commands: ...").
                    let in_list = blocks[sents[i].block].kind == BlockKind::ListItem;
                    let tokens = erg.tokens(text);
                    let (parse, skipped) = if !opts.parse {
                        (None, Some("parsing disabled".to_string()))
                    } else if tokens.len() > opts.max_tokens {
                        (
                            None,
                            Some(format!(
                                "{} tokens; longer than {}",
                                tokens.len(),
                                opts.max_tokens
                            )),
                        )
                    } else if lead_in(text) {
                        (None, Some("a lead-in to what follows".to_string()))
                    } else {
                        let masked = parser_text(text, in_list);
                        let text = masked.as_deref().unwrap_or(text);
                        match timing::timed(&timing::PARSE, || {
                            erg.parse_limited(text, opts.timeout, opts.max_readings)
                        }) {
                            Ok(p) => (Some(p), None),
                            Err(e) => (None, Some(e.to_string())),
                        }
                    };
                    // Readings that quote without quoting (see
                    // `unlicensed_quoting`) are no evidence that the
                    // sentence is grammatical. When every strict reading is
                    // one, they are dropped, after making sure no other
                    // strict reading was left out by the reading limit or
                    // by chart pruning.
                    let parse = parse.map(|p| {
                        let masked = parser_text(text, in_list);
                        let text = masked.as_deref().unwrap_or(text);
                        let quoting_only = |p: &Parse| {
                            let strict: Vec<&Reading> = p
                                .readings
                                .iter()
                                .filter(|r| emdysi_parse::is_strict(&r.root))
                                .collect();
                            !strict.is_empty() && strict.iter().all(|r| unlicensed_quoting(r, text))
                        };
                        if !quoting_only(&p) {
                            return p;
                        }
                        let p = if p.readings.len() >= opts.max_readings {
                            match timing::timed(&timing::REPARSE, || {
                                erg.parse_limited(text, opts.timeout, ALL_READINGS)
                            }) {
                                Ok(q) if q.readings.len() < ALL_READINGS && quoting_only(&q) => q,
                                _ => return p,
                            }
                        } else {
                            p
                        };
                        // A pruned search proves nothing about what it
                        // missed: search again without pruning, which, if
                        // it finishes, shows whether another analysis
                        // exists.
                        let p = if p.complete {
                            p
                        } else {
                            match timing::timed(&timing::REPARSE, || {
                                erg.parse_unpruned(text, opts.timeout, ALL_READINGS)
                            }) {
                                Ok(q)
                                    if q.complete
                                        && q.readings.len() < ALL_READINGS
                                        && quoting_only(&q) =>
                                {
                                    q
                                }
                                _ => p,
                            }
                        };
                        let mut p = p;
                        p.readings.retain(|r| !unlicensed_quoting(r, text));
                        p
                    });
                    let strict = parse.as_ref().is_some_and(|p| {
                        p.readings.iter().any(|r| emdysi_parse::is_strict(&r.root))
                    });
                    // Unknown words are analysed generically, so a wrong
                    // form ("buyed") can still look grammatical.
                    let generic = parse.as_ref().is_some_and(|p| {
                        p.readings
                            .first()
                            .is_some_and(|r| r.words.iter().any(|w| w.generic))
                    });
                    // The grammar-error variant is only asked for readings
                    // with few corrections (see `rules::grammar_errors`), so
                    // a short search suffices.
                    // With a strict analysis, an error counts only within
                    // an unknown word other than an acronym or masked code
                    // (see `rules::grammar_errors`): when there is none, the
                    // grammar-error variant is not asked.
                    let generic = generic
                        && (!strict
                            || parse.as_ref().is_some_and(|p| {
                                let original: Vec<char> = sents[i].original.chars().collect();
                                let masked: Vec<char> = parser_text(&sents[i].original, false)
                                    .unwrap_or_default()
                                    .chars()
                                    .collect();
                                p.readings.first().is_some_and(|r| {
                                    r.words.iter().any(|w| {
                                        w.generic
                                            && !acronym(&original, w.from, w.to)
                                            && !masked_code(&masked, w.from, w.to)
                                    })
                                })
                            }));
                    // Without a strict analysis, errors are reported only when
                    // a complete search found fragments only.
                    let proved = parse.as_ref().is_some_and(|q| {
                        q.complete && q.readings.iter().all(|r| r.root == "fragment")
                    });
                    let mal_parse = if opts.diagnose
                        && parse.is_some()
                        && (if strict {
                            generic
                        } else {
                            proved || opts.meaning
                        }) {
                        let masked = parser_text(text, in_list);
                        let text = masked.as_deref().unwrap_or(text);
                        erg.mal().and_then(|m| {
                            timing::timed(&timing::MAL, || {
                                m.parse_limited(
                                    text,
                                    opts.timeout.min(MAL_TIMEOUT),
                                    opts.max_readings.min(MAL_READINGS),
                                )
                            })
                            .ok()
                            .map(|mut p| {
                                p.readings.retain(|r| !unlicensed_quoting(r, text));
                                p
                            })
                        })
                    } else {
                        None
                    };
                    results
                        .lock()
                        .unwrap()
                        .push((i, parse, tokens, skipped, mal_parse));
                }
            });
        }
    });
    for (i, parse, tokens, skipped, mal_parse) in results.into_inner().unwrap() {
        sents[i].mal_parse = mal_parse;
        sents[i].parse = parse;
        sents[i].tokens = tokens;
        sents[i].skipped = skipped;
    }
    Analysis {
        source: source.to_string(),
        format,
        blocks,
        sentences: sents,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Error,
    Warning,
    Suggestion,
}

impl Severity {
    pub fn parse(s: &str) -> Option<Severity> {
        match s {
            "error" => Some(Severity::Error),
            "warning" => Some(Severity::Warning),
            "suggestion" | "info" => Some(Severity::Suggestion),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Suggestion => "suggestion",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub rule: String,
    pub severity: Severity,
    pub message: String,
    /// Byte range in the source.
    pub range: Range<usize>,
    /// Text to put in place of `range`, for automatic fixes.
    pub replacement: Option<String>,
    /// Other candidate replacements, not applied automatically.
    pub suggestions: Vec<String>,
    pub sentence: Option<usize>,
    /// A finding too uncertain to report on its own (a word two edits from
    /// its suggestion, which may be a rare correct word): kept only when
    /// the decision model confirms it, dropped without one.
    pub needs_model: bool,
}

/// Runs rule packs over analyses.
pub struct Checker {
    pub packs: Vec<Pack>,
    /// Rule ids (or `pack.*` prefixes) to skip.
    pub disabled: Vec<String>,
}

impl Checker {
    /// The concepts of all packs.
    pub fn glossary(&self) -> terms::Glossary {
        terms::Glossary {
            concepts: self
                .packs
                .iter()
                .flat_map(|p| p.concepts.iter().cloned())
                .collect(),
        }
    }
}

impl Checker {
    pub fn new(packs: Vec<Pack>) -> Self {
        Checker {
            packs,
            disabled: Vec::new(),
        }
    }

    fn enabled(&self, id: &str) -> bool {
        !self.disabled.iter().any(|d| match d.strip_suffix('*') {
            Some(prefix) => id.starts_with(prefix),
            None => d == id,
        })
    }

    pub fn check(&self, erg: &Erg, a: &Analysis) -> Vec<Diagnostic> {
        self.check_with(erg, a, None)
    }

    /// [`Checker::check`], with a decision model for `decide` rules
    /// (without one they are skipped).
    pub fn check_with(
        &self,
        erg: &Erg,
        a: &Analysis,
        mut ask: Option<&mut (dyn decisions::Ask + '_)>,
    ) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        let glossary = self.glossary();
        for pack in &self.packs {
            for rule in &pack.rules {
                if self.enabled(&rule.id) {
                    let ask = ask.as_mut().map(|x| &mut **x as &mut dyn decisions::Ask);
                    let t = std::time::Instant::now();
                    rule.run_with(erg, a, &glossary, ask, &mut out);
                    let d = t.elapsed();
                    timing::RULES
                        .fetch_add(d.as_nanos() as u64, std::sync::atomic::Ordering::Relaxed);
                    *timing::RULE_TIMES
                        .lock()
                        .unwrap()
                        .entry(rule.id.clone())
                        .or_default() += d;
                }
            }
        }
        // Rules over the other rules' findings.
        let mut over = Vec::new();
        for pack in &self.packs {
            for rule in &pack.rules {
                if self.enabled(&rule.id) {
                    rule.run_density(a, &out, &mut over);
                }
            }
        }
        out.extend(over);
        out.sort_by(|x, y| {
            x.range
                .start
                .cmp(&y.range.start)
                .then(x.severity.cmp(&y.severity))
                .then(x.rule.cmp(&y.rule))
        });
        out.dedup_by(|x, y| x.rule == y.rule && x.range == y.range);
        out
    }
}

/// Apply the automatic fixes of non-overlapping diagnostics to the source.
pub fn apply_fixes(source: &str, diags: &[Diagnostic]) -> (String, usize) {
    let mut fixes: Vec<(&Range<usize>, &str)> = diags
        .iter()
        .filter_map(|d| d.replacement.as_deref().map(|r| (&d.range, r)))
        .collect();
    fixes.sort_by_key(|(r, _)| (r.start, r.end));
    let mut out = String::with_capacity(source.len());
    let mut pos = 0;
    let mut applied = 0;
    for (r, rep) in fixes {
        if r.start < pos || r.end > source.len() {
            continue;
        }
        out.push_str(&source[pos..r.start]);
        out.push_str(rep);
        pos = r.end;
        applied += 1;
    }
    out.push_str(&source[pos..]);
    (out, applied)
}

#[cfg(test)]
mod tests {
    use super::parser_text;

    #[test]
    fn code_like_tokens_are_masked() {
        let t = "Open src/main.rs and see crates.io or mail security@rust-lang.org.";
        let m = parser_text(t, false).unwrap();
        assert_eq!(m.chars().count(), t.chars().count());
        let mask = |w: &str| format!("X{}", "x".repeat(w.chars().count() - 1));
        assert_eq!(
            m,
            format!(
                "Open {} and see {} or mail {}.",
                mask("src/main.rs"),
                mask("crates.io"),
                mask("security@rust-lang.org")
            )
        );
        // Ordinary words with a slash or dots are left alone.
        assert_eq!(parser_text("We met at 5 p.m. and/or later.", false), None);
        assert_eq!(
            parser_text("The U.S. economy grew, e.g. in May.", false),
            None
        );
    }

    #[test]
    fn citations_are_masked() {
        let t = "People who sleep remember more (Walker, 2017).";
        let m = parser_text(t, false).unwrap();
        assert_eq!(m.chars().count(), t.chars().count());
        let blank = |c: &str| " ".repeat(c.chars().count());
        assert_eq!(m, t.replace("(Walker, 2017)", &blank("(Walker, 2017)")));
        let t = "Others report none [3, 5-7].";
        assert_eq!(
            parser_text(t, false).unwrap(),
            t.replace("[3, 5-7]", &blank("[3, 5-7]"))
        );
        let t = "Results (see Smith and Jones 2020, p. 4) vary.";
        assert!(parser_text(t, false).unwrap().starts_with("Results    "));
        // Not citations.
        assert!(parser_text("It costs $5 (about 4 euros).", false).is_none());
        assert!(parser_text("Version 2 (released in March) is out.", false).is_none());
        // Labels only in list items.
        assert!(parser_text("Keep going: stay motivated and keep pushing.", false).is_none());
        assert!(parser_text("Keep going: stay motivated and keep pushing.", true).is_some());
    }
}
