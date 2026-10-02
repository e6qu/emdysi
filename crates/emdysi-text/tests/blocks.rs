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
