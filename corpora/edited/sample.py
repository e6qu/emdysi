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

# The held-out test set (heldout-*.tsv): other documents from the same
# sources, never looked at while developing the checks; see SOURCE.md.
HELDOUT_FICTION_CHAPTERS = range(7, 31)
HELDOUT_GOV_DIRS = ["concise", "web", "design", "test", "conversational", "audience", "organize"]
HELDOUT_RUST_PAGES = [
    "src/ch01-02-hello-world.md",
    "src/ch01-03-hello-cargo.md",
    "src/ch03-02-data-types.md",
    "src/ch03-03-how-functions-work.md",
    "src/ch04-02-references-and-borrowing.md",
    "src/ch05-01-defining-structs.md",
    "src/ch06-01-defining-an-enum.md",
    "src/ch07-01-packages-and-crates.md",
    "src/ch08-01-vectors.md",
    "src/ch09-02-recoverable-errors-with-result.md",
]
HELDOUT_K8S_PAGES = [
    "content/en/docs/concepts/overview/kubernetes-api.md",
    "content/en/docs/concepts/overview/working-with-objects/_index.md",
    "content/en/docs/concepts/overview/working-with-objects/labels.md",
    "content/en/docs/concepts/overview/working-with-objects/annotations.md",
    "content/en/docs/concepts/overview/working-with-objects/names.md",
    "content/en/docs/concepts/overview/working-with-objects/namespaces.md",
    "content/en/docs/concepts/overview/working-with-objects/finalizers.md",
    "content/en/docs/concepts/overview/working-with-objects/owners-dependents.md",
    "content/en/docs/concepts/overview/working-with-objects/field-selectors.md",
    "content/en/docs/concepts/extend-kubernetes/_index.md",
    "content/en/docs/concepts/extend-kubernetes/operator.md",
    "content/en/docs/concepts/extend-kubernetes/api-extension/custom-resources.md",
    "content/en/docs/concepts/containers/_index.md",
    "content/en/docs/concepts/containers/images.md",
    "content/en/docs/concepts/containers/container-environment.md",
    "content/en/docs/concepts/containers/runtime-class.md",
    "content/en/docs/concepts/policy/resource-quotas.md",
    "content/en/docs/concepts/policy/limit-range.md",
    "content/en/docs/concepts/workloads/_index.md",
    "content/en/docs/concepts/workloads/controllers/replicaset.md",
    "content/en/docs/concepts/workloads/controllers/job.md",
    "content/en/docs/concepts/workloads/controllers/cron-jobs.md",
    "content/en/docs/concepts/architecture/_index.md",
    "content/en/docs/concepts/architecture/controller.md",
]
HELDOUT_BLOG_POSTS = [
    "content/Async-await-hits-beta.md",
    "content/Async-await-stable.md",
    "content/Cargo.md",
    "content/Fearless-Concurrency.md",
    "content/Fearless-Concurrency-In-Firefox-Quantum.md",
    "content/Increasing-Apple-Version-Requirements.md",
    "content/Increasing-glibc-kernel-requirements.md",
    "content/Mozilla-IRC-Sunset-and-the-Rust-Channel.md",
    "content/Next-year.md",
    "content/Planning-2021-Roadmap.md",
    "content/Rust-2018-dev-tools.md",
    "content/Rust-2021-public-testing.md",
    "content/Rust-2024-public-testing.md",
    "content/Rust-Once-Run-Everywhere.md",
    "content/Rust-Roadmap-Update.md",
    "content/RustConf.md",
    "content/Scheduling-2021-Roadmap.md",
    "content/Security-advisory-for-std.md",
    "content/Core-Team.md",
    "content/Enums-match-mutation-and-moves.md",
]


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


# A second, larger development set (dev2-*.tsv): yet other documents of
# the same sources, chosen by rule from what is left (see `dev2` below).
DEV2_FICTION_CHAPTERS = range(31, 62)
DEV2_BOOK_CHAPTERS = ("ch10-", "ch11-", "ch12-", "ch13-", "ch15-", "ch16-")
DEV2_K8S_PAGES = 30
DEV2_BLOG_POSTS = 25


def fiction(up, chapters=FICTION_CHAPTERS, prefix="fiction"):
    text = (up / "Pride-and-Prejudice_1342" / "1342-0.txt").read_text(encoding="utf-8-sig")
    text = text.replace("\r\n", "\n")
    body = text.split("*** START OF THIS PROJECT GUTENBERG EBOOK PRIDE AND PREJUDICE ***", 1)[1]
    body = body.split("*** END OF THIS PROJECT GUTENBERG EBOOK", 1)[0]
    parts = re.split(r"\n(Chapter \d+)\n", body)
    rows = []
    for i in range(1, len(parts) - 1, 2):
        n = int(parts[i].split()[1])
        if n not in chapters:
            continue
        paras = [" ".join(p.split()) for p in parts[i + 1].split("\n\n") if p.strip()]
        rows.append((f"{prefix}-{n:02}", "fiction", "1342-0.txt", "plain", "\n\n".join(paras) + "\n", ""))
    return rows


def pages(up, repo, paths, genre, prefix):
    rows = []
    for k, p in enumerate(paths, 1):
        s = clean_markdown((up / repo / p).read_text(encoding="utf-8"))
        rows.append((f"{prefix}-{k:02}", genre, p, "md", s, ""))
    return rows


def conllu(up, repo, name, genre, prefix, limit, skip=0):
    rows = []
    seen = 0
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
            seen += 1
            if seen <= skip:
                continue
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
    gov_dev = set(GOV_PAGES)
    gov_heldout = sorted(
        str(p.relative_to(up / "plainlanguage.gov"))
        for d in HELDOUT_GOV_DIRS
        for p in (up / "plainlanguage.gov" / "_pages" / "guidelines" / d).glob("*.md")
        if str(p.relative_to(up / "plainlanguage.gov")) not in gov_dev and p.name != "index.md"
    )
    outputs += [
        (here / "heldout-fiction.tsv", fiction(up, HELDOUT_FICTION_CHAPTERS, "hfiction")),
        (here / "heldout-government.tsv", pages(up, "plainlanguage.gov", gov_heldout, "government", "hgov")),
        (
            here / "heldout-rust.tsv",
            pages(up, "book", HELDOUT_RUST_PAGES, "technical", "hrust")
            + pages(up, "blog", HELDOUT_BLOG_POSTS, "blog", "hblog"),
        ),
        (here / "heldout-kubernetes.tsv", pages(up, "website", HELDOUT_K8S_PAGES, "technical", "hk8s")),
        (
            here.parent / "edited-by-sa" / "heldout-pud.tsv",
            conllu(up, "UD_English-PUD", "en_pud-ud-test.conllu", "news-wiki", "hpud", 500, skip=MAX_SENTENCES),
        ),
    ]
    # dev2: what neither set uses, chosen by rule (sorted, the first N).
    used_k8s = set(K8S_PAGES) | set(HELDOUT_K8S_PAGES)
    k8s_all = sorted(
        str(p.relative_to(up / "website"))
        for p in (up / "website" / "content" / "en" / "docs" / "concepts").rglob("*.md")
    )
    dev2_k8s = [p for p in k8s_all if p not in used_k8s][:DEV2_K8S_PAGES]
    dev2_book = sorted(
        f"src/{p.name}"
        for p in (up / "book" / "src").glob("ch*.md")
        if p.name.startswith(DEV2_BOOK_CHAPTERS)
    )
    used_blog = set(BLOG_POSTS) | set(HELDOUT_BLOG_POSTS)
    release = re.compile(r"(?i)release|^Rust-1\.|^1\.\d|survey|results")
    dev2_blog = [
        p
        for p in sorted(
            f"content/{q.name}" for q in (up / "blog" / "content").glob("*.md") if not release.search(q.name)
        )
        if p not in used_blog
    ][:DEV2_BLOG_POSTS]
    dev2_gov = sorted(
        str(p.relative_to(up / "plainlanguage.gov"))
        for p in (up / "plainlanguage.gov" / "_pages" / "guidelines" / "words").glob("*.md")
        if p.name != "index.md"
    )
    outputs += [
        (here / "dev2-fiction.tsv", fiction(up, DEV2_FICTION_CHAPTERS, "dfiction")),
        (here / "dev2-government.tsv", pages(up, "plainlanguage.gov", dev2_gov, "government", "dgov")),
        (
            here / "dev2-rust.tsv",
            pages(up, "book", dev2_book, "technical", "drust") + pages(up, "blog", dev2_blog, "blog", "dblog"),
        ),
        (here / "dev2-kubernetes.tsv", pages(up, "website", dev2_k8s, "technical", "dk8s")),
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
