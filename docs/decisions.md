# Decisions

Recorded 2026-10-01 after the [prior-art survey](prior-art.md).

| # | Topic | Decision |
|---|---|---|
| D1 | Project license | MIT. |
| D2 | Dependency licenses | Apache-2.0 crates are allowed. **Only dependencies whose license has been verified from the primary source may be used** (see [dependencies.md](dependencies.md)). |
| D3 | POS tagging | Rule-based tagger for v1 (lexicon candidates + hand-written disambiguation). No statistical models trained on LDC or NC data. |
| D4 | Grammar | **Full port of the English Resource Grammar.** We implement a DELPH-IN-compatible processor in Rust (TDL reader, typed feature structures, unification, morphology, REPP, chart mapping, chart parser) and run the vendored ERG unchanged. Phrase-structure trees come from the ERG's own node labels (`parse-nodes.tdl`). |
| D5 | Input/output formats | Input: plain text and Markdown. Output: plain text and Markdown. |
| D6 | Test corpora | May be vendored under `corpora/`, one directory per corpus with its license and provenance, only if redistribution is permitted (see [corpora/README.md](../corpora/README.md)). |

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
