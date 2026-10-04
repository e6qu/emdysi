# Edited text in several genres (sample)

Published, edited English for measuring how often the checker claims an
error in text that has none (`crates/emdysi-check/examples/false_flags.rs`).
The samples (`fiction.tsv`, `government.tsv`, `rust.tsv` and
`kubernetes.tsv`, one per license; same columns as described in
`sample.py`) are built by `sample.py` from the upstream repositories below,
checked out at the commits given, retrieved 2026-10-04. Text is copied as
is; only markup is removed (front matter, template tags, HTML tags and
comments) and, for the novel, line breaks within paragraphs are joined and
the Project Gutenberg header and footer are dropped. Licenses were read in
each repository.

| Rows | Genre | Upstream | Commit | Files | License | Copyright |
|---|---|---|---|---|---|---|
| `fiction-01` to `-06` (`fiction.tsv`) | fiction | https://github.com/GITenberg/Pride-and-Prejudice_1342 | `81db45c9c48c592f0b77f01fc59e677ad0a5634e` | `1342-0.txt`, chapters 1 to 6 | Public domain (Jane Austen, 1813; Project Gutenberg eBook #1342) | none |
| `gov-01` to `-10` (`government.tsv`) | government | https://github.com/GSA/plainlanguage.gov | `fd7694740f19c0ed20ed71c2dd1dc92699e920fa` | ten pages under `_pages/guidelines/` (see `sample.py`) | Public domain in the US and CC0 1.0 worldwide (`LICENSE-plainlanguage.md`) | none (work of the US Government, GSA) |
| `rust-01` to `-04` (`rust.tsv`) | technical | https://github.com/rust-lang/book | `1500248d8f230566e4ec9f27fcbb8fe9e2898ab1` | four chapters under `src/` | MIT OR Apache-2.0 (`LICENSE-rust-book-MIT`, `LICENSE-rust-book-APACHE`) | The Rust Project Developers (Steve Klabnik, Carol Nichols, Chris Krycho and contributors) |
| `k8s-01` to `-04` (`kubernetes.tsv`) | technical | https://github.com/kubernetes/website | `77db41e9c776b614fdb31de4cc6c8e9a70673817` | four pages under `content/en/docs/concepts/` | CC BY 4.0 (`LICENSE-kubernetes-website`) | The Kubernetes Authors |
| `blog-01` to `-06` (`rust.tsv`) | blog | https://github.com/rust-lang/blog.rust-lang.org | `c7510dd334a8690c207a08e3e4b6a17a8b675bf2` | six posts under `content/` | MIT OR Apache-2.0 (README: "the blog is licensed MIT/Apache 2.0"; `LICENSE-rust-blog-MIT`, `LICENSE-rust-blog-APACHE`) | The Rust Project Developers and the posts' authors |

Share-alike text (UD English PUD, news and Wikipedia) is kept apart in
[`../edited-by-sa`](../edited-by-sa/SOURCE.md). UD English EWT (web text)
was considered and not used: only its annotations are CC BY-SA; its text
comes from the LDC English Web Treebank (LDC2012T13), with portions
copyright Google, Yahoo! and the University of Pennsylvania.
