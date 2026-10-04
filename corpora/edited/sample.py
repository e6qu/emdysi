#!/usr/bin/env python3
"""Build the samples in corpora/edited/ (one TSV file per license) and
corpora/edited-by-sa/pud.tsv from pinned upstream checkouts. The
share-alike (CC BY-SA) text goes to its own directory, as
corpora/README.md requires.

Usage: python3 corpora/edited/sample.py UPSTREAM_DIR

UPSTREAM_DIR holds clones of the repositories listed in SOURCE.md, checked
out at the commits given there, under their repository names
(plainlanguage.gov, Pride-and-Prejudice_1342, book, website, blog,
UD_English-PUD).

Output columns (tab-separated; backslash, tab, newline and carriage return
escaped as \\\\, \\t, \\n, \\r): id, genre, source file, format (md or
plain), text, and annotated errors ("form>correction" separated by " | ",
from Typo=Yes and CorrectForm in a UD treebank; none in these sources).
"""

import re
import sys
from pathlib import Path

MAX_SENTENCES = 500  # per UD treebank

GOV_PAGES = [
    "_pages/guidelines/concise/write-short-sentences.md",
    "_pages/guidelines/concise/write-short-paragraphs.md",
    "_pages/guidelines/concise/use-positive-language.md",
    "_pages/guidelines/concise/keep-the-subject-verb-and-object-close-together.md",
    "_pages/guidelines/web/write-effective-links.md",
    "_pages/guidelines/web/avoid-faqs.md",
    "_pages/guidelines/design/minimize-cross-references.md",
    "_pages/guidelines/design/highlight-important-concepts.md",
    "_pages/guidelines/test/paraphrase-testing.md",
    "_pages/guidelines/test/usability-testing.md",
]
RUST_PAGES = [
    "src/ch00-00-introduction.md",
    "src/ch01-01-installation.md",
    "src/ch03-01-variables-and-mutability.md",
    "src/ch04-01-what-is-ownership.md",
]
BLOG_POSTS = [
    "content/4-Years-Of-Rust.md",
    "content/A-call-for-blogs-2020.md",
    "content/2025-09-crates-io-phishing-campaign.md",
    "content/RLS-deprecation.md",
    "content/Procedural-Macros-in-Rust-2018.md",
    "content/inside-rust/what-is-maintenance-anyway.md",
]
K8S_PAGES = [
    "content/en/docs/concepts/overview/_index.md",
    "content/en/docs/concepts/overview/components.md",
    "content/en/docs/concepts/architecture/nodes.md",
    "content/en/docs/concepts/workloads/pods/_index.md",
]
FICTION_CHAPTERS = range(1, 7)


def esc(s):
    return (
        s.replace("\\", "\\\\")
        .replace("\t", "\\t")
        .replace("\n", "\\n")
        .replace("\r", "\\r")
    )


def strip_front_matter(s):
    for fence in ("---", "+++"):
        if s.startswith(fence):
            end = s.find("\n" + fence, 3)
            if end >= 0:
                s = s[end + 4 :]
    return s


def clean_markdown(s):
    s = strip_front_matter(s)
    # Jekyll/Liquid and Hugo template tags, HTML comments and tags.
    s = re.sub(r"\{%.*?%\}", "", s, flags=re.S)
    # Hugo shortcodes that stand for words: a glossary tooltip is its text
    # (or its term), the version parameters a version number.
    def tooltip(m):
        t = re.search(r'text="([^"]*)"', m.group(0))
        if t:
            return t.group(1)
        t = re.search(r'term_id="([^"]*)"', m.group(0))
        return t.group(1).replace("-", " ") if t else ""

    s = re.sub(r"\{\{<\s*glossary_tooltip[^>]*>\}\}", tooltip, s)
    s = re.sub(r'\{\{<\s*(?:param "version"|skew currentVersion)\s*>\}\}', "v1.34", s)
    s = re.sub(r"\{\{[<%].*?[>%]\}\}", "", s, flags=re.S)
    # Markdown autolinks keep their address.
    s = re.sub(r"<((?:https?://|mailto:)?[^<>\s]+@[^<>\s]+|https?://[^<>\s]+)>", r"\1", s)
    s = re.sub(r"\{\{.*?\}\}", "", s, flags=re.S)
    s = re.sub(r"<!--.*?-->", "", s, flags=re.S)
    s = re.sub(r"</?[a-zA-Z][^>]*>", "", s)
    # Kramdown attribute lists such as {:.class}.
    s = re.sub(r"\{:[^}]*\}", "", s)
    s = re.sub(r"\n{3,}", "\n\n", s)
    return s.strip() + "\n"


def fiction(up):
    text = (up / "Pride-and-Prejudice_1342" / "1342-0.txt").read_text(encoding="utf-8-sig")
    text = text.replace("\r\n", "\n")
    body = text.split("*** START OF THIS PROJECT GUTENBERG EBOOK PRIDE AND PREJUDICE ***", 1)[1]
    body = body.split("*** END OF THIS PROJECT GUTENBERG EBOOK", 1)[0]
    parts = re.split(r"\n(Chapter \d+)\n", body)
    rows = []
    for i in range(1, len(parts) - 1, 2):
        n = int(parts[i].split()[1])
        if n not in FICTION_CHAPTERS:
            continue
        paras = [" ".join(p.split()) for p in parts[i + 1].split("\n\n") if p.strip()]
        rows.append((f"fiction-{n:02}", "fiction", "1342-0.txt", "plain", "\n\n".join(paras) + "\n", ""))
    return rows


def pages(up, repo, paths, genre, prefix):
    rows = []
    for k, p in enumerate(paths, 1):
        s = clean_markdown((up / repo / p).read_text(encoding="utf-8"))
        rows.append((f"{prefix}-{k:02}", genre, p, "md", s, ""))
    return rows


def conllu(up, repo, name, genre, prefix, limit):
    rows = []
    doc, para, errors, n = [], [], [], 0
    doc_id = None

    def flush_para():
        if para:
            doc.append(" ".join(para))
            para.clear()

    def flush_doc():
        flush_para()
        if doc:
            rows.append(
                (f"{prefix}-{len(rows) + 1:03}", genre, f"{name} {doc_id}", "plain", "\n\n".join(doc) + "\n", " | ".join(errors))
            )
        doc.clear()
        errors.clear()

    for line in (up / repo / name).read_text(encoding="utf-8").splitlines():
        if line.startswith("# newdoc"):
            if n >= limit:
                break
            flush_doc()
            doc_id = line.split("=", 1)[1].strip()
        elif line.startswith("# newpar"):
            flush_para()
        elif line.startswith("# text = "):
            para.append(line[len("# text = ") :])
            n += 1
        elif line and line[0].isdigit():
            f = line.split("\t")
            if len(f) == 10 and "Typo=Yes" in f[5]:
                m = re.search(r"CorrectForm=([^|]*)", f[9])
                if m:
                    errors.append(f"{f[1]}>{m.group(1)}")
    flush_doc()
    return rows


def main():
    up = Path(sys.argv[1])
    here = Path(__file__).parent
    # One file per license, so each has one entry in VENDORED.toml.
    outputs = [
        (here / "fiction.tsv", fiction(up)),
        (here / "government.tsv", pages(up, "plainlanguage.gov", GOV_PAGES, "government", "gov")),
        (
            here / "rust.tsv",
            pages(up, "book", RUST_PAGES, "technical", "rust")
            + pages(up, "blog", BLOG_POSTS, "blog", "blog"),
        ),
        (here / "kubernetes.tsv", pages(up, "website", K8S_PAGES, "technical", "k8s")),
        (
            here.parent / "edited-by-sa" / "pud.tsv",
            conllu(up, "UD_English-PUD", "en_pud-ud-test.conllu", "news-wiki", "pud", MAX_SENTENCES),
        ),
    ]
    for out, rows in outputs:
        out.parent.mkdir(exist_ok=True)
        with out.open("w", encoding="utf-8") as f:
            f.write("# id\tgenre\tsource\tformat\ttext\tannotated errors\n")
            for r in rows:
                f.write("\t".join(esc(x) for x in r) + "\n")
        print(f"wrote {len(rows)} documents to {out}")


if __name__ == "__main__":
    main()
