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

pub mod dict;
pub mod report;
pub mod rules;
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
            .is_some_and(|p| p.readings.iter().any(|r| r.root == "root_strict"))
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
}

impl Default for Options {
    fn default() -> Self {
        Options {
            parse: true,
            threads: std::thread::available_parallelism().map_or(2, |n| n.get()),
            max_tokens: 40,
            timeout: Duration::from_secs(10),
        }
    }
}

/// A sentence's parse outcome: index, parse, tokens, reason for skipping.
type Parsed = (usize, Option<Parse>, Vec<InputToken>, Option<String>);

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
                    } else {
                        match erg.parse_with_timeout(text, opts.timeout) {
                            Ok(p) => (Some(p), None),
                            Err(e) => (None, Some(e.to_string())),
                        }
                    };
                    results.lock().unwrap().push((i, parse, tokens, skipped));
                }
            });
        }
    });
    for (i, parse, tokens, skipped) in results.into_inner().unwrap() {
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
}

/// Runs rule packs over analyses.
pub struct Checker {
    pub packs: Vec<Pack>,
    /// Rule ids (or `pack.*` prefixes) to skip.
    pub disabled: Vec<String>,
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
        let mut out = Vec::new();
        for pack in &self.packs {
            for rule in &pack.rules {
                if self.enabled(&rule.id) {
                    rule.run(erg, a, &mut out);
                }
            }
        }
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
