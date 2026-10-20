//! Prose blocks of a document, with a map back to source offsets.
//!
//! Markdown is read with pulldown-cmark: paragraphs, headings, list items,
//! block quotes and table cells become blocks; code blocks and HTML blocks
//! become empty `Code` blocks (so that document-structure checks see them
//! but no prose check does), except HTML blocks made only of paragraphs
//! with inline markup, which are prose; image descriptions are skipped; inline code is
//! kept but marked opaque so that checks can leave it alone. Plain text is
//! split into paragraphs at blank lines.

use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Paragraph,
    Heading(u8),
    ListItem,
    Quote,
    TableCell,
    /// A code or HTML block: no text, only its position.
    Code,
}

/// A run of prose. `text` is what checks see; `offsets[i]` is the source
/// byte offset of byte `i` of `text` (one extra entry maps the end).
#[derive(Debug, Clone)]
pub struct Block {
    pub kind: BlockKind,
    pub text: String,
    pub offsets: Vec<usize>,
    /// Ranges of `text` that are not prose (inline code, URLs).
    pub opaque: Vec<Range<usize>>,
    /// The list this block belongs to (numbered from 1; 0 if none), whether
    /// that list is numbered, and the list item (numbered from 1 across the
    /// document; 0 if none). A list item with several paragraphs gives
    /// several blocks with the same `item`.
    pub list: usize,
    pub ordered: bool,
    pub item: usize,
}

impl Block {
    fn new(kind: BlockKind) -> Self {
        Block {
            kind,
            text: String::new(),
            offsets: Vec::new(),
            opaque: Vec::new(),
            list: 0,
            ordered: false,
            item: 0,
        }
    }

    fn push(&mut self, s: &str, source: Option<usize>, fallback: usize) {
        for (i, _) in s.bytes().enumerate() {
            self.offsets.push(source.map_or(fallback, |b| b + i));
        }
        self.text.push_str(s);
    }

    /// Source byte range of a range of `text`.
    pub fn source_range(&self, r: Range<usize>) -> Range<usize> {
        let start = self.offsets.get(r.start).copied().unwrap_or(0);
        let end = if r.end == 0 {
            start
        } else {
            self.offsets.get(r.end - 1).map_or(start, |&e| e + 1)
        };
        start..end.max(start)
    }

    pub fn is_opaque(&self, r: &Range<usize>) -> bool {
        self.opaque
            .iter()
            .any(|o| o.start < r.end && r.start < o.end)
    }
}

/// Split plain text into paragraphs at blank lines.
pub fn plain_blocks(src: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut cur: Option<Block> = None;
    let mut pos = 0;
    for line in src.split_inclusive('\n') {
        let start = pos;
        pos += line.len();
        if line.trim().is_empty() {
            if let Some(b) = cur.take() {
                out.push(finish(b));
            }
            continue;
        }
        let b = cur.get_or_insert_with(|| Block::new(BlockKind::Paragraph));
        b.push(line, Some(start), start);
    }
    if let Some(b) = cur.take() {
        out.push(finish(b));
    }
    out
}

/// Hugo shortcodes in `b`'s text: |{{< glossary_tooltip text="API
/// resources" term_id="api-resource" >}}| is its `text` (prose that
/// renders), any other (|{{< note >}}|, |{{% /tab %}}|) is dropped. The
/// offset map and the opaque ranges follow.
fn shortcodes(b: &mut Block) {
    if !b.text.contains("{{") {
        return;
    }
    // The byte ranges of the old text to keep, in order.
    let mut keep: Vec<Range<usize>> = Vec::new();
    let mut at = 0;
    while let Some(i) = b.text[at..].find("{{").map(|i| at + i) {
        let rest = &b.text[i + 2..];
        // (Inside inline code, a shortcode is code: |`{{< note >}}`|.)
        let in_code = b.is_opaque(&(i..i + 1));
        let Some(open) = rest
            .chars()
            .next()
            .filter(|c| matches!(c, '<' | '%') && !in_code)
        else {
            keep.push(at..i + 2);
            at = i + 2;
            continue;
        };
        let close = if open == '<' { ">}}" } else { "%}}" };
        let Some(end) = rest.find(close).map(|e| i + 2 + e + close.len()) else {
            break;
        };
        keep.push(at..i);
        let inner = &b.text[i + 3..end - 3];
        if inner.trim_start().starts_with("glossary_tooltip") {
            // The value of |text="..."| (straight or curly quotes).
            if let Some(v) = inner.find("text=").map(|v| i + 3 + v + 5) {
                let q = b.text[v..].chars().next().unwrap_or(' ');
                if matches!(q, '"' | '“' | '”') {
                    let from = v + q.len_utf8();
                    if let Some(len) = b.text[from..end].find(['"', '“', '”']) {
                        keep.push(from..from + len);
                    }
                }
            }
        }
        at = end;
    }
    keep.push(at..b.text.len());
    let mut text = String::new();
    let mut offsets = Vec::new();
    // Old byte index -> new byte index, for the bytes kept.
    let mut map = vec![None; b.text.len() + 1];
    for r in keep.iter().filter(|r| r.start < r.end) {
        for (k, j) in r.clone().enumerate() {
            map[j] = Some(text.len() + k);
        }
        text.push_str(&b.text[r.clone()]);
        offsets.extend_from_slice(&b.offsets[r.clone()]);
    }
    map[b.text.len()] = Some(text.len());
    b.opaque = b
        .opaque
        .iter()
        .filter_map(|o| {
            let kept: Vec<usize> = o.clone().filter_map(|j| map[j]).collect();
            Some(*kept.first()?..*kept.last()? + 1)
        })
        .collect();
    b.text = text;
    b.offsets = offsets;
}

fn finish(mut b: Block) -> Block {
    shortcodes(&mut b);
    // Trim trailing whitespace but keep the offset map aligned.
    let trimmed = b.text.trim_end().len();
    b.text.truncate(trimmed);
    b.offsets.truncate(trimmed);
    b
}

/// Inline HTML elements whose text is prose (`code` is kept opaque).
const INLINE_HTML: &[&str] = &["a", "b", "code", "em", "i", "span", "strong", "br"];

/// The paragraphs of an HTML block made only of `<p>` elements with inline
/// markup (|<p><code>caCertPath</code> is the path ...</p>|, as in generated
/// reference pages), as blocks; `None` for any other HTML. Entities are
/// decoded, and each character maps to its source offset (`base` + its
/// position in `html`).
fn html_paragraphs(html: &str, base: usize) -> Option<Vec<Block>> {
    let mut out = Vec::new();
    let mut cur: Option<Block> = None;
    let mut code_from: Option<usize> = None;
    let mut rest = html;
    let mut at = 0;
    while !rest.is_empty() {
        if let Some(tag) = rest.strip_prefix('<') {
            let end = tag.find('>')?;
            let inner = &tag[..end];
            let closing = inner.starts_with('/');
            let name: String = inner
                .trim_start_matches('/')
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect::<String>()
                .to_ascii_lowercase();
            if name == "p" {
                if closing {
                    out.push(cur.take()?);
                } else if cur.is_some() {
                    return None;
                } else {
                    cur = Some(Block::new(BlockKind::Paragraph));
                }
            } else if INLINE_HTML.contains(&name.as_str()) {
                let b = cur.as_mut()?;
                if name == "br" {
                    b.push(" ", None, base + at);
                } else if name == "code" {
                    if closing {
                        let from = code_from.take()?;
                        b.opaque.push(from..b.text.len());
                    } else {
                        code_from = Some(b.text.len());
                    }
                }
            } else {
                return None;
            }
            at += end + 2;
            rest = &rest[end + 2..];
            continue;
        }
        let c = rest.chars().next()?;
        let (text, len) = if c == '&' {
            let semi = rest.find(';').filter(|&i| i <= 8)?;
            let decoded = match &rest[1..semi] {
                "quot" => "\"",
                "amp" => "&",
                "lt" => "<",
                "gt" => ">",
                "apos" | "#39" => "'",
                "nbsp" => " ",
                _ => return None,
            };
            (decoded.to_string(), semi + 1)
        } else {
            (c.to_string(), c.len_utf8())
        };
        match cur.as_mut() {
            Some(b) => b.push(&text, (len == c.len_utf8()).then_some(base + at), base + at),
            // Only whitespace between paragraphs.
            None if c.is_whitespace() => {}
            None => return None,
        }
        at += len;
        rest = &rest[len..];
    }
    (cur.is_none() && code_from.is_none() && !out.is_empty()).then_some(out)
}

/// Prose blocks of a Markdown document.
pub fn markdown_blocks(src: &str) -> Vec<Block> {
    // Front matter (YAML between `---` lines, or Hugo's TOML between
    // `+++` lines) is metadata, not prose.
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS;
    let mut out = Vec::new();
    let mut cur: Option<Block> = None;
    let mut quote_depth = 0;
    let mut item_depth = 0;
    // Open lists (id, numbered) and the current item, innermost last.
    let mut lists: Vec<(usize, bool)> = Vec::new();
    let mut items: Vec<usize> = Vec::new();
    let (mut list_count, mut item_count) = (0, 0);
    // Inside code blocks, HTML blocks or image descriptions: skip text.
    let mut skip = 0;
    let flush = |cur: &mut Option<Block>, out: &mut Vec<Block>| {
        if let Some(b) = cur.take() {
            let b = finish(b);
            if !b.text.trim().is_empty() {
                out.push(b);
            }
        }
    };
    let kind_here = |quote: usize, item: usize| {
        if item > 0 {
            BlockKind::ListItem
        } else if quote > 0 {
            BlockKind::Quote
        } else {
            BlockKind::Paragraph
        }
    };
    let new_block = |kind: BlockKind, lists: &[(usize, bool)], items: &[usize]| {
        let mut b = Block::new(kind);
        if kind == BlockKind::ListItem {
            if let (Some(&(l, o)), Some(&i)) = (lists.last(), items.last()) {
                (b.list, b.ordered, b.item) = (l, o, i);
            }
        }
        b
    };
    for (ev, range) in Parser::new_ext(src, opts).into_offset_iter() {
        match ev {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {
                    flush(&mut cur, &mut out);
                    cur = Some(new_block(
                        kind_here(quote_depth, item_depth),
                        &lists,
                        &items,
                    ));
                }
                Tag::Heading { level, .. } => {
                    flush(&mut cur, &mut out);
                    cur = Some(Block::new(BlockKind::Heading(level as u8)));
                }
                Tag::List(start) => {
                    flush(&mut cur, &mut out);
                    list_count += 1;
                    lists.push((list_count, start.is_some()));
                }
                Tag::Item => {
                    flush(&mut cur, &mut out);
                    item_depth += 1;
                    item_count += 1;
                    items.push(item_count);
                    cur = Some(new_block(BlockKind::ListItem, &lists, &items));
                }
                Tag::BlockQuote(_) => {
                    flush(&mut cur, &mut out);
                    quote_depth += 1;
                }
                Tag::TableCell => {
                    flush(&mut cur, &mut out);
                    cur = Some(Block::new(BlockKind::TableCell));
                }
                Tag::HtmlBlock => {
                    flush(&mut cur, &mut out);
                    // Paragraphs of plain HTML are prose; any other HTML
                    // is opaque, like code.
                    match html_paragraphs(&src[range.clone()], range.start) {
                        Some(ps) => out.extend(ps.into_iter().map(finish)),
                        None => {
                            let mut b = Block::new(BlockKind::Code);
                            b.offsets.push(range.start);
                            out.push(b);
                        }
                    }
                    skip += 1
                }
                Tag::CodeBlock(_) => {
                    flush(&mut cur, &mut out);
                    let mut b = Block::new(BlockKind::Code);
                    b.offsets.push(range.start);
                    out.push(b);
                    skip += 1
                }
                Tag::Image { .. } | Tag::MetadataBlock(_) => skip += 1,
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::TableCell => {
                    flush(&mut cur, &mut out)
                }
                TagEnd::Item => {
                    flush(&mut cur, &mut out);
                    item_depth -= 1;
                    items.pop();
                }
                TagEnd::List(_) => {
                    flush(&mut cur, &mut out);
                    lists.pop();
                }
                TagEnd::BlockQuote(_) => {
                    flush(&mut cur, &mut out);
                    quote_depth -= 1;
                }
                TagEnd::CodeBlock
                | TagEnd::HtmlBlock
                | TagEnd::Image
                | TagEnd::MetadataBlock(_) => skip -= 1,
                _ => {}
            },
            Event::Text(t) if skip == 0 => {
                let b = cur.get_or_insert_with(|| {
                    new_block(kind_here(quote_depth, item_depth), &lists, &items)
                });
                let exact = src.get(range.clone()) == Some(&*t);
                b.push(&t, exact.then_some(range.start), range.start);
            }
            Event::Code(t) if skip == 0 => {
                let b = cur.get_or_insert_with(|| {
                    new_block(kind_here(quote_depth, item_depth), &lists, &items)
                });
                let start = b.text.len();
                // Map the code text into the source between its backticks.
                let inner = src
                    .get(range.clone())
                    .and_then(|s| s.find(&*t).map(|p| range.start + p));
                b.push(&t, inner, range.start);
                b.opaque.push(start..b.text.len());
            }
            Event::SoftBreak | Event::HardBreak if skip == 0 => {
                // A line that is only a bold label (|**Service discovery**|
                // over the text it introduces) ends a block of its own,
                // rather than running into the next line's sentence.
                if bold_label_line(src, range.start) {
                    flush(&mut cur, &mut out);
                } else if let Some(b) = cur.as_mut() {
                    b.push(" ", None, range.start);
                }
            }
            _ => {}
        }
    }
    flush(&mut cur, &mut out);
    out
}

/// Whether the source line that ends at `at` is only a bold label: strong
/// emphasis (`**…**` or `__…__`), perhaps with a colon, after any list
/// marker.
fn bold_label_line(src: &str, at: usize) -> bool {
    let start = src[..at.min(src.len())].rfind('\n').map_or(0, |i| i + 1);
    let line = src[start..at.min(src.len())].trim();
    let line = line
        .strip_prefix(['*', '-', '+'])
        .map(str::trim_start)
        .filter(|rest| rest.starts_with("**") || rest.starts_with("__"))
        .unwrap_or_else(|| {
            let digits = line.trim_start_matches(|c: char| c.is_ascii_digit());
            match digits.strip_prefix(['.', ')']) {
                Some(rest) if digits.len() < line.len() => rest.trim_start(),
                _ => line,
            }
        });
    let line = line.strip_suffix(':').unwrap_or(line).trim_end();
    ["**", "__"].iter().any(|m| {
        line.len() > 2 * m.len()
            && line.starts_with(m)
            && line.ends_with(m)
            && !line[m.len()..line.len() - m.len()].contains(m)
    })
}
