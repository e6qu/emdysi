# Decisions

Recorded 2026-10-01 after the [prior-art survey](prior-art.md).

| # | Topic | Decision |
|---|---|---|
| D1 | Project license | MIT. |
| D2 | Dependency licenses | Apache-2.0 crates are allowed. **Only dependencies whose license has been verified from the primary source may be used** (see [dependencies.md](dependencies.md)). |
| D3 | POS tagging | Rule-based tagger for v1 (lexicon candidates + hand-written disambiguation). No statistical models trained on LDC or NC data. |
| D4 | Grammar | **Full port of the English Resource Grammar.** We implement a DELPH-IN-compatible processor in Rust (TDL reader, typed feature structures, unification, morphology, REPP, chart mapping, chart parser) and run the vendored ERG unchanged. Phrase-structure trees come from the ERG's own node labels (`parse-nodes.tdl`). |
| D5 | Input/output formats | Input: plain text and Markdown. Output: plain text and Markdown. |
| D7 | Command name | The command-line tool is `en` (no common command uses that name). The package stays `emdysi` because the crates.io name `en` is taken. |
| D8 | Spelling variants | Both American and British spellings are accepted (British *-ise* and *-ize* forms). |
| D6 | Test corpora | May be vendored under `corpora/`, one directory per corpus with its license and provenance, only if redistribution is permitted (see [corpora/README.md](../corpora/README.md)). |
| D9 | Traceability | Every vendored or non-code file (grammar, corpora, data, trained weights) has an entry in [`VENDORED.toml`](../VENDORED.toml): upstream, exact version, retrieval date, SPDX license, license file, copyright holders, provenance of the text, modifications and use. [`VENDORED.sha256`](../VENDORED.sha256) pins every file's content. `scripts/check-vendored.py` enforces both in CI: an unlisted, changed or unlicensed file fails the build. |
| D10 | Rewriting with a local model | A small local model (optional, never vendored; run in-process from a GGUF file through llama.cpp, or served locally over the OpenAI-compatible API, e.g. by MLX on Apple silicon) may **rewrite** sentences, guided and checked by every deterministic and statistical layer: the rewrite must parse with the ERG (strict root), keep the meaning (MRS of the rewrite equivalent to the original's, modulo the intended edit), introduce no new diagnostics from the rule packs or spelling, and win under the parse ranker and the model's own likelihood. Diagnostics and the grammar's analysis go into the prompt; candidates that fail a check are discarded or repaired, never shown. The same cascade fixes typos: candidates come from the word list (edit distance, commonness), the grammar's lexicon and morphology, and the model; the ERG keeps only candidates with which the sentence parses, and the ranker and the model's likelihood in context choose among them. |
| D11 | Style-guide rules | Rules that enforce style guides (document structure, terminology, compounds) read the grammar's analysis rather than surface patterns wherever the distinction depends on grammar (a verb *set up* against a noun *setup*, an *-ly* adverb against an *-ly* adjective, a noun stack against a name). Every rule carries `examples` it must flag and `acceptable` documents it must not, run as tests, and a `source` naming the style guide or tool it follows. Rule data adapted from other tools is imported only under a license on the D2 allowlist (no CC BY-SA lists such as Wikipedia's "Signs of AI writing" or GitLab's Vale rules, no LanguageTool rule data). A project's established jargon is declared once, in a TBX-style glossary (`--glossary`), and is then exempt from the coined-word, noun-stack and acronym rules. Machine-generated corpora record their `generator` and its terms in `VENDORED.toml` and are vendored per row and column, keeping only text whose license is clear. |

## Consequences

- The ERG relies on a TnT POS tagger for unknown words and a maximum-entropy
  model (`redwoods.mem`) for parse ranking. Both are statistical models trained
  on corpora that include LDC text, so neither is vendored. The rule-based
  tagger (D3) feeds the ERG's token-mapping rules (`+TNT` tags), and ranking
  starts heuristic. Whether `redwoods.mem` may be used (it is distributed in
  the MIT-licensed ERG repository, but trained on WSJ-derived treebanks) is an
  open question. Instead, `crates/emdysi-parse/data/rank.tsv` is our own
  model, trained only on the vendored gold profiles (ERG-authored test
  suites and a public-domain story); see `examples/train.rs`.
- The ERG is large (about 7.5k types, 44k lexical entries, 290 syntactic
  rules), so engine performance (quick-check, rule filter, packing) is part of
  the port, not an afterthought.
