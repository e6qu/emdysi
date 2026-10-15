use emdysi_text::blocks::*;

#[test]
fn markdown_prose_and_offsets() {
    let src = "# Title here\n\nSome *emphasis* and `code()` text.\nNext line.\n\n```\nnot prose\n```\n\n- item one\n- item **two**\n\n> quoted text\n";
    let blocks = markdown_blocks(src);
    let texts: Vec<(&str, BlockKind)> = blocks.iter().map(|b| (b.text.as_str(), b.kind)).collect();
    assert_eq!(
        texts,
        vec![
            ("Title here", BlockKind::Heading(1)),
            (
                "Some emphasis and code() text. Next line.",
                BlockKind::Paragraph
            ),
            ("", BlockKind::Code),
            ("item one", BlockKind::ListItem),
            ("item two", BlockKind::ListItem),
            ("quoted text", BlockKind::Quote),
        ]
    );
    // The two items belong to one bulleted list.
    assert_eq!(
        (blocks[3].list, blocks[3].ordered, blocks[3].item),
        (1, false, 1)
    );
    assert_eq!((blocks[4].list, blocks[4].item), (1, 2));
    assert_eq!(blocks[5].list, 0);
    let p = &blocks[1];
    let at = p.text.find("emphasis").unwrap();
    let r = p.source_range(at..at + "emphasis".len());
    assert_eq!(&src[r], "emphasis");
    let at = p.text.find("code()").unwrap();
    assert!(p.is_opaque(&(at..at + 1)));
    assert_eq!(&src[p.source_range(at..at + 6)], "code()");
}

#[test]
fn plain_paragraphs() {
    let src = "One. Two.\nThree.\n\n\nFour.\n";
    let blocks = plain_blocks(src);
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].text, "One. Two.\nThree.");
    let r = blocks[1].source_range(0..5);
    assert_eq!(&src[r], "Four.");
}

#[test]
fn html_paragraphs_are_prose() {
    // A paragraph of plain HTML, as in generated reference pages: its text
    // is prose, inline code is opaque, entities are decoded, and offsets
    // point into the source.
    let src = "<p><code>caCertPath</code> is the path to the &quot;CA&quot; file.</p>\n";
    let blocks = markdown_blocks(src);
    assert_eq!(blocks.len(), 1);
    let b = &blocks[0];
    assert_eq!(b.kind, BlockKind::Paragraph);
    assert_eq!(b.text, "caCertPath is the path to the \"CA\" file.");
    assert_eq!(b.opaque, vec![0..10]);
    let at = b.text.find("path").unwrap();
    assert_eq!(&src[b.offsets[at]..b.offsets[at] + 4], "path");
    // Any other HTML stays opaque.
    let blocks = markdown_blocks("<div class=\"x\">Not prose here.</div>\n");
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].kind, BlockKind::Code);
    let blocks = markdown_blocks("<table><tr><td>cell</td></tr></table>\n");
    assert_eq!(blocks[0].kind, BlockKind::Code);
}
