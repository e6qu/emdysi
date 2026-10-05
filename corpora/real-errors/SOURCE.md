# Real errors from documentation commit histories

Errors that writers made and fixed in published documentation: each row
is a paragraph before and after a commit whose message mentions a typo,
spelling, grammar or a misspelling, mined with [`mine.py`](mine.py), which
describes the selection and the three kinds (`spelling`,
`grammar-function-word`, `grammar-inflection`). Used to measure how many
real errors the checker catches (`crates/emdysi-check/examples/real_errors.rs`).
The kinds are assigned automatically and some pairs are not errors a
grammar can know about (a word replaced for its meaning), so recall
measured here is a lower bound.

| File | Upstream | Commit (history up to) | Selection | License | Copyright |
|---|---|---|---|---|---|
| `rust-book.tsv` | https://github.com/rust-lang/book | `1500248d8f230566e4ec9f27fcbb8fe9e2898ab1` (2026-09-02) | `mine.py REPO src/ OUT` (all 262 matching commits; 122 pairs) | MIT OR Apache-2.0 (`LICENSE-rust-book-MIT`, `LICENSE-rust-book-APACHE`); used under MIT | The Rust Project Developers and the book's contributors |
| `kubernetes.tsv` | https://github.com/kubernetes/website | `27a415c72e2ca106a48145ea3ffa620486ccda14` (2026-10-05) | `mine.py REPO content/en/ OUT 400` (the 400 most recent matching commits; 255 pairs) | CC BY 4.0 (`LICENSE-kubernetes-website`) | The Kubernetes Authors |

Retrieved 2026-10-05. The text before each fix is part of the repository's
history and under the repository's license, as is the text after it; no
text is changed.
