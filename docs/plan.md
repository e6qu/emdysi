# Plan

The work follows the pipeline in [prior-art.md](prior-art.md#proposed-architecture-first-cut),
with the ERG port (decision D4) as the parsing core.

| Milestone | Scope | Status |
|---|---|---|
| M0 | Workspace, decisions, license policy, vendored ERG, test corpora | done |
| M1 | `emdysi-tdl`: TDL reader that loads the whole ERG | done |
| M2 | Type hierarchy: multiple inheritance, addenda, GLB closure, feature appropriateness | done |
| M3 | Typed feature structures: unification, copying, subsumption; well-typed expansion of every ERG type, rule and lexical entry | unification, copying and expansion done; subsumption pending |
| M4 | REPP tokenizer (`rpp/`) and token lattice with character spans | REPP done |
| M5 | Chart mapping: token-mapping rules (`tmr/`) and lexical filtering (`lfr.tdl`) | done |
| M6 | Orthographic morphology (`%suffix`/`%prefix`, letter sets, `irregs.tab`) and lexical lookup, including generic entries for unknown words | done (multiword entries inflect on the last word) |
| M7 | Rule-based POS tagger feeding `+TNT` tags (decision D3) | first heuristic version |
| M8 | Chart parser: agenda, rule filter, quick-check, ambiguity packing, unpacking, root conditions | agenda, quick-check (incl. two-step for binary rules), packing, unpacking, root conditions done |
| M9 | Output: derivation trees, labelled phrase-structure trees, sentence type; plain-text and Markdown rendering | derivations and labelled trees done |
| M10 | Parse selection (heuristic first) and robust fallback for fragments | |
| M11 | Sentence segmentation and Markdown input with span mapping | done (46/47 Golden Rules) |
| M12 | Spelling: SCOWL-based lexicon, suggestions, confident auto-fix | |
| M13 | Rules engine and DSL; AI-prose and style-guide packs | |
| M14 | CLI | |

Validation: the gold derivations in `corpora/erg-gold/` (ERG `mrs` and `csli`
test suites) are the reference for M3–M9. The engine should reproduce the
gold tree among its analyses and reject the items marked ungrammatical.
