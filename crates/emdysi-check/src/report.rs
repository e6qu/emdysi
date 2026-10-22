//! Reports: plain text and Markdown for people, JSON and SARIF for
//! programs.

use std::fmt::Write as _;

use crate::{Analysis, Diagnostic, Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Plain,
    Markdown,
    /// One JSON object per document (see [`render`]).
    Json,
    /// SARIF 2.1.0, one log for all documents (see [`Sarif`]).
    Sarif,
}

/// `s` as a JSON string, quoted and escaped.
pub fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_list(items: impl IntoIterator<Item = String>) -> String {
    format!("[{}]", items.into_iter().collect::<Vec<_>>().join(","))
}

/// A diagnostic as a JSON object: where it is (byte offsets, and lines
/// and columns from 1, columns in characters, the end exclusive), the
/// text found there, and the replacement applied by `en fix` (`fix`) or
/// the other candidates (`suggestions`), each to put in place of the
/// whole range.
fn json_diagnostic(a: &Analysis, d: &Diagnostic) -> String {
    let (line, column) = a.line_col(d.range.start);
    let (end_line, end_column) = a.line_col(d.range.end);
    let found = a.source.get(d.range.clone()).unwrap_or("");
    format!(
        "{{\"rule\":{},\"severity\":{},\"message\":{},\"start\":{},\"end\":{},\"line\":{line},\"column\":{column},\"end_line\":{end_line},\"end_column\":{end_column},\"text\":{},\"fix\":{},\"suggestions\":{}}}",
        json_str(&d.rule),
        json_str(d.severity.as_str()),
        json_str(&d.message),
        d.range.start,
        d.range.end,
        json_str(found),
        d.replacement.as_deref().map_or("null".into(), json_str),
        json_list(d.suggestions.iter().map(|s| json_str(s))),
    )
}

/// The diagnostics of several documents as one SARIF 2.1.0 log, for code
/// review and CI tools: [`Sarif::add`] each document, then
/// [`Sarif::render`].
#[derive(Default)]
pub struct Sarif {
    results: Vec<String>,
    rules: std::collections::BTreeSet<String>,
}

impl Sarif {
    pub fn add(&mut self, a: &Analysis, diags: &[Diagnostic], name: &str) {
        for d in diags {
            self.rules.insert(d.rule.clone());
            let (line, column) = a.line_col(d.range.start);
            let (end_line, end_column) = a.line_col(d.range.end);
            let region = format!(
                "{{\"startLine\":{line},\"startColumn\":{column},\"endLine\":{end_line},\"endColumn\":{end_column}}}"
            );
            let location = format!("{{\"uri\":{}}}", json_str(name));
            let level = match d.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Suggestion => "note",
            };
            let fixes = match &d.replacement {
                Some(r) => format!(
                    ",\"fixes\":[{{\"description\":{{\"text\":{}}},\"artifactChanges\":[{{\"artifactLocation\":{location},\"replacements\":[{{\"deletedRegion\":{region},\"insertedContent\":{{\"text\":{}}}}}]}}]}}]",
                    json_str(&format!("Replace with {r:?}")),
                    json_str(r)
                ),
                None => String::new(),
            };
            self.results.push(format!(
                "{{\"ruleId\":{},\"level\":\"{level}\",\"message\":{{\"text\":{}}},\"locations\":[{{\"physicalLocation\":{{\"artifactLocation\":{location},\"region\":{region}}}}}]{fixes}}}",
                json_str(&d.rule),
                json_str(&d.message),
            ));
        }
    }

    pub fn render(&self) -> String {
        let rules = json_list(
            self.rules
                .iter()
                .map(|r| format!("{{\"id\":{}}}", json_str(r))),
        );
        format!(
            "{{\"version\":\"2.1.0\",\"$schema\":\"https://json.schemastore.org/sarif-2.1.0.json\",\"runs\":[{{\"tool\":{{\"driver\":{{\"name\":\"emdysi\",\"informationUri\":\"https://github.com/e6qu/emdysi\",\"version\":{},\"rules\":{rules}}}}},\"columnKind\":\"unicodeCodePoints\",\"results\":{}}}]}}\n",
            json_str(env!("CARGO_PKG_VERSION")),
            json_list(self.results.iter().cloned()),
        )
    }
}

/// `n` and a noun in the singular or the plural (|1 warning|, |2
/// warnings|).
pub fn count(n: usize, singular: &str, plural: &str) -> String {
    format!("{n} {}", if n == 1 { singular } else { plural })
}

fn counts(diags: &[Diagnostic]) -> String {
    let n = |s: Severity| diags.iter().filter(|d| d.severity == s).count();
    format!(
        "{}, {}, {}",
        count(n(Severity::Error), "error", "errors"),
        count(n(Severity::Warning), "warning", "warnings"),
        count(n(Severity::Suggestion), "suggestion", "suggestions")
    )
}

/// The source line containing a byte offset, and the column range of
/// `range` on it (in characters).
fn excerpt(a: &Analysis, range: &std::ops::Range<usize>) -> (String, usize, usize) {
    let src = &a.source;
    let start = range.start.min(src.len());
    let line_start = src[..start].rfind('\n').map_or(0, |i| i + 1);
    let line_end = src[start..].find('\n').map_or(src.len(), |i| start + i);
    let line = &src[line_start..line_end];
    let col = src[line_start..start].chars().count();
    let width = src[start..range.end.min(line_end).max(start)]
        .chars()
        .count()
        .max(1);
    (line.to_string(), col, width)
}

pub fn render(a: &Analysis, diags: &[Diagnostic], name: &str, format: OutputFormat) -> String {
    let mut out = String::new();
    match format {
        OutputFormat::Plain => {
            for d in diags {
                let (line, col) = a.line_col(d.range.start);
                let _ = writeln!(
                    out,
                    "{name}:{line}:{col}: {} [{}] {}",
                    d.severity.as_str(),
                    d.rule,
                    d.message
                );
                let (text, c, w) = excerpt(a, &d.range);
                let _ = writeln!(out, "    {text}");
                let _ = writeln!(out, "    {}{}", " ".repeat(c), "^".repeat(w));
                if let Some(rep) = &d.replacement {
                    let _ = writeln!(out, "    fix: {rep:?}");
                } else if !d.suggestions.is_empty() {
                    let _ = writeln!(out, "    suggestions: {}", d.suggestions.join(", "));
                }
            }
            let _ = writeln!(out, "{name}: {}", counts(diags));
        }
        OutputFormat::Json => {
            let n = |s: Severity| diags.iter().filter(|d| d.severity == s).count();
            let _ = writeln!(
                out,
                "{{\"file\":{},\"counts\":{{\"error\":{},\"warning\":{},\"suggestion\":{}}},\"diagnostics\":{}}}",
                json_str(name),
                n(Severity::Error),
                n(Severity::Warning),
                n(Severity::Suggestion),
                json_list(diags.iter().map(|d| json_diagnostic(a, d))),
            );
        }
        OutputFormat::Sarif => {
            let mut sarif = Sarif::default();
            sarif.add(a, diags, name);
            out.push_str(&sarif.render());
        }
        OutputFormat::Markdown => {
            let _ = writeln!(out, "## `{name}`\n");
            let _ = writeln!(out, "{}\n", counts(diags));
            for d in diags {
                let (line, col) = a.line_col(d.range.start);
                let found = &a.source[d.range.clone()];
                let found = found.replace('`', "'").replace('\n', " ");
                let _ = write!(
                    out,
                    "- **{}** `{}` (line {line}, column {col}): {}",
                    d.severity.as_str(),
                    d.rule,
                    d.message
                );
                let _ = write!(out, " — `{found}`");
                if let Some(rep) = &d.replacement {
                    let _ = write!(out, " → `{}`", rep.replace('`', "'"));
                } else if !d.suggestions.is_empty() {
                    let _ = write!(out, " (suggestions: {})", d.suggestions.join(", "));
                }
                out.push('\n');
            }
        }
    }
    out
}

/// What `render_parses` shows besides the phrase-structure tree.
#[derive(Debug, Clone, Copy, Default)]
pub struct ParseDetails {
    pub derivations: bool,
    /// The semantics (MRS) of the best reading.
    pub mrs: bool,
}

/// Sentence-by-sentence parse report.
pub fn render_parses(a: &Analysis, format: OutputFormat, show: &ParseDetails) -> String {
    if matches!(format, OutputFormat::Json | OutputFormat::Sarif) {
        return json_parses(a, show);
    }
    let derivations = show.derivations;
    let mut out = String::new();
    for (i, s) in a.sentences.iter().enumerate() {
        let (line, _) = a.line_col(a.sentence_source(i).start);
        let n = s.parse.as_ref().map_or(0, |p| p.readings.len());
        let status = match (&s.skipped, n, s.strict()) {
            (Some(why), _, _) => format!("not parsed: {why}"),
            (None, 0, _) => "no analysis".to_string(),
            (None, n, true) => count(n, "reading", "readings"),
            (None, n, false) => format!(
                "{}, fragment or informal only",
                count(n, "reading", "readings")
            ),
        };
        let best = s.best();
        let tree = best.and_then(|r| r.tree.as_ref()).map(|t| t.bracketed());
        let ambiguity = ambiguity_lines(s);
        match format {
            OutputFormat::Plain => {
                let _ = writeln!(
                    out,
                    "[{}] line {line}: {}",
                    i + 1,
                    s.original.replace('\n', " ")
                );
                let _ = writeln!(out, "    {status}");
                if let Some(t) = &tree {
                    let _ = writeln!(out, "    {t}");
                }
                for l in &ambiguity {
                    let _ = writeln!(out, "    {l}");
                }
                if derivations {
                    if let Some(r) = best {
                        let _ = writeln!(out, "    {}", r.derivation);
                    }
                }
                if show.mrs {
                    if let Some(m) = best.and_then(|r| r.mrs.as_ref()) {
                        let _ = writeln!(out, "    {}", m.to_simple());
                    }
                }
            }
            // JSON and SARIF returned above.
            _ => {
                let _ = writeln!(
                    out,
                    "{}. {} *(line {line}; {status})*",
                    i + 1,
                    s.original.replace('\n', " ")
                );
                if let Some(t) = &tree {
                    let _ = writeln!(out, "\n   ```\n   {t}\n   ```");
                }
                if !ambiguity.is_empty() {
                    let _ = writeln!(out);
                    for l in &ambiguity {
                        let _ = writeln!(out, "   {l}");
                    }
                }
                if derivations {
                    if let Some(r) = best {
                        let _ = writeln!(out, "\n   ```\n   {}\n   ```", r.derivation);
                    }
                }
                if show.mrs {
                    if let Some(m) = best.and_then(|r| r.mrs.as_ref()) {
                        let _ = writeln!(out, "\n   ```\n   {}\n   ```", m.to_simple());
                    }
                }
            }
        }
    }
    out
}

/// The sentences of a document as one JSON object (`{"sentences": [...]}`
/// on one line): each with its line, text, whether it was parsed and
/// with a strict (full, grammatical) analysis, the number of readings, the
/// best reading's tree, and how the sentence is ambiguous; derivations
/// and semantics as asked.
fn json_parses(a: &Analysis, show: &ParseDetails) -> String {
    let sentences = a.sentences.iter().enumerate().map(|(i, s)| {
        let (line, _) = a.line_col(a.sentence_source(i).start);
        let n = s.parse.as_ref().map_or(0, |p| p.readings.len());
        let best = s.best();
        let opt = |x: Option<String>| x.as_deref().map_or("null".into(), json_str);
        let mut o = format!(
            "{{\"index\":{},\"line\":{line},\"text\":{},\"skipped\":{},\"readings\":{n},\"strict\":{},\"tree\":{},\"ambiguity\":{}",
            i + 1,
            json_str(&s.original),
            opt(s.skipped.as_ref().map(|w| w.to_string())),
            n > 0 && s.strict(),
            opt(best.and_then(|r| r.tree.as_ref()).map(|t| t.bracketed())),
            json_list(ambiguity_lines(s).iter().map(|l| json_str(l))),
        );
        if show.derivations {
            let _ = write!(o, ",\"derivation\":{}", opt(best.map(|r| r.derivation.clone())));
        }
        if show.mrs {
            let _ = write!(
                o,
                ",\"mrs\":{}",
                opt(best.and_then(|r| r.mrs.as_ref()).map(|m| m.to_simple()))
            );
        }
        o.push('}');
        o
    });
    format!("{{\"sentences\":{}}}\n", json_list(sentences))
}

/// Lines that say how a sentence is ambiguous: the interpretations (see
/// [`emdysi_parse::ambiguity`]) of its full readings that keep at least
/// [`emdysi_parse::ambiguity::MIN_SHARE`] of the probability, each with the
/// dependencies that set it apart from the most probable one. Empty when
/// the sentence has one interpretation.
pub fn ambiguity_lines(s: &crate::Sentence) -> Vec<String> {
    interpretations(s, emdysi_parse::ambiguity::MIN_SHARE)
        .map(|kept| lines(&kept))
        .unwrap_or_default()
}

/// The shares of a sentence's interpretations that keep at least
/// `min_share` of the probability, and what sets the second one apart from
/// the first (`with(man, telescope) instead of with(saw, telescope)`);
/// `None` when the sentence has only one such interpretation.
pub fn ambiguity(s: &crate::Sentence, min_share: f64) -> Option<(Vec<f64>, String)> {
    let kept = interpretations(s, min_share)?;
    let only = kept[1].differences(&kept[0]);
    let instead = kept[0].differences(&kept[1]);
    let alt = format!(
        "{} instead of {}",
        if only.is_empty() {
            "-".into()
        } else {
            only.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
        },
        if instead.is_empty() {
            "-".into()
        } else {
            instead
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        }
    );
    Some((kept.iter().map(|i| i.probability).collect(), alt))
}

/// The interpretations of a sentence's full readings (all readings when it
/// has none) that keep at least `min_share`, if there are two or more.
fn interpretations(
    s: &crate::Sentence,
    min_share: f64,
) -> Option<Vec<emdysi_parse::ambiguity::Interpretation>> {
    let p = s.parse.as_ref()?;
    let strict = s.strict();
    let readings: Vec<emdysi_parse::Reading> = p
        .readings
        .iter()
        .filter(|r| !strict || emdysi_parse::is_strict(&r.root))
        .cloned()
        .collect();
    let all = emdysi_parse::ambiguity::interpretations(&readings, p.temperature, &s.original);
    let kept: Vec<_> = all
        .into_iter()
        .filter(|i| i.probability >= min_share)
        .collect();
    (kept.len() >= 2).then_some(kept)
}

fn lines(kept: &[emdysi_parse::ambiguity::Interpretation]) -> Vec<String> {
    let pct = |x: f64| format!("{:.0}%", 100.0 * x);
    let mut out = vec![format!(
        "ambiguous: {} interpretations ({})",
        kept.len(),
        kept.iter()
            .map(|i| pct(i.probability))
            .collect::<Vec<_>>()
            .join(", ")
    )];
    let first = &kept[0];
    for (k, i) in kept.iter().enumerate().skip(1) {
        let mut only: Vec<String> = i.differences(first);
        let mut instead: Vec<String> = first.differences(i);
        only.truncate(4);
        instead.truncate(4);
        out.push(format!(
            "  {}. {}: {} instead of {}",
            k + 1,
            pct(i.probability),
            if only.is_empty() {
                "-".into()
            } else {
                only.join(", ")
            },
            if instead.is_empty() {
                "-".into()
            } else {
                instead.join(", ")
            }
        ));
    }
    out
}
