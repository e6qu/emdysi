//! Plain-text and Markdown reports.

use std::fmt::Write as _;

use crate::{Analysis, Diagnostic, Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Plain,
    Markdown,
}

fn counts(diags: &[Diagnostic]) -> String {
    let n = |s: Severity| diags.iter().filter(|d| d.severity == s).count();
    format!(
        "{} errors, {} warnings, {} suggestions",
        n(Severity::Error),
        n(Severity::Warning),
        n(Severity::Suggestion)
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
    let derivations = show.derivations;
    let mut out = String::new();
    for (i, s) in a.sentences.iter().enumerate() {
        let (line, _) = a.line_col(a.sentence_source(i).start);
        let n = s.parse.as_ref().map_or(0, |p| p.readings.len());
        let status = match (&s.skipped, n, s.strict()) {
            (Some(why), _, _) => format!("not parsed: {why}"),
            (None, 0, _) => "no analysis".to_string(),
            (None, n, true) => format!("{n} readings"),
            (None, n, false) => format!("{n} readings, fragment or informal only"),
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
            OutputFormat::Markdown => {
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
