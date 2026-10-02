//! Prose blocks of a document, with a map back to source offsets.
//!
//! Markdown is read with pulldown-cmark: paragraphs, headings, list items,
//! block quotes and table cells become blocks; code blocks and HTML blocks
//! become empty `Code` blocks (so that document-structure checks see them
//! but no prose check does); image descriptions are skipped; inline code is
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

fn finish(mut b: Block) -> Block {
    // Trim trailing whitespace but keep the offset map aligned.
    let trimmed = b.text.trim_end().len();
    b.text.truncate(trimmed);
    b.offsets.truncate(trimmed);
    b
}

/// Prose blocks of a Markdown document.
pub fn markdown_blocks(src: &str) -> Vec<Block> {
    let opts = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;
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
                Tag::CodeBlock(_) | Tag::HtmlBlock => {
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
                if let Some(b) = cur.as_mut() {
                    b.push(" ", None, range.start);
                }
            }
            _ => {}
        }
    }
    flush(&mut cur, &mut out);
    out
}
