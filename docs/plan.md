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
| M8 | Chart parser: agenda, rule filter, quick-check, ambiguity packing, unpacking, root conditions | agenda (shorter spans first), quick-check (incl. two-step for binary rules), packing, unpacking, root conditions done; chart pruning for inputs over 20 positions (the ranker's local features score edges, 40 kept per span) and best-first unpacking |
| M9a | Semantics: MRS read-out (VPM, SimpleMRS, isomorphism, well-formedness checks) | done: identical to the gold MRS for all 2,966 gold analyses reproduced (mrs, csli, esd, ccs, control, sh-spec) |
| M9b | Named grammatical errors with the ERG's mal-rule variant (`educ/`) | done: on csli, 104 of 388 ungrammatical items get a named error and 70 a generic one; 17 of 965 grammatical items get a false named error |
| M9 | Output: derivation trees, labelled phrase-structure trees, sentence type; plain-text and Markdown rendering | derivations and labelled trees done |
| M9d | Semantic prose checks from the MRS: missing comparand, agentless passive, stacked negation, bare demonstrative, tense shift | done (`semantics` rule kind) |
| M9e | Guarded rewriting and typo correction (decision D10): candidates from fixes, spelling and an optional local model (llama.cpp, GGUF), kept only when the grammar accepts them, the MRS content is preserved and no new diagnostic appears; `en rewrite` | done; the model path is tested with a scripted stand-in (no weights can be downloaded in this environment) |
| M9c | Minimal-pair suites (BLiMP and Zorro samples) as grammaticality tests | done: see [evaluation.md](evaluation.md); morphosyntax 87-100% of pairs, semantic/pragmatic paradigms left to the statistical and model layers |
| M10 | Parse selection and robust fallback for fragments | averaged-perceptron ranker on clean gold: 81.7% exact match held out on 2,626 items of all lengths (77.9% when trained on the earlier, short-only set); the ERG's fragment/informal roots act as fallback, then a cover of the input by the fewest partial analyses |
| M11 | Sentence segmentation and Markdown input with span mapping | done (46/47 Golden Rules) |
| M12 | Spelling: SCOWL-based lexicon, suggestions, confident auto-fix | done |
| M13 | Rules engine and DSL; AI-prose and style-guide packs | 25 rule kinds, 6 default and 4 opt-in packs; rule scopes (heading, lead, body, ...) and per-rule examples run as tests |
| M13b | Document structure and terminology (research: `reports/AI corpora and style guide rules.md`, not in the repository): heading hierarchy, bottom line up front, paragraph and section size, parallel headings and lists; acronym definitions (Schwartz–Hearst), a TBX-style glossary, one spelling per term, coined words and concept names, hyphen chains, -ly hyphens, noun stacks | done (`structure` and `terms` packs) |
| M13c | Rule data imported from other linters under permissive licenses: Vale's Microsoft, Google and Elastic packages and the words/* lists, converted by `scripts/import-rules.py` into opt-in packs (`existence` and `substitution` kinds) | done; proselint, Red Hat, GitLab and LanguageTool data excluded for licensing reasons (see rules.md) |
| M13d | Opening paragraphs and procedures: `vague-lead` (no concrete claim in the semantics of the opening), numbered steps that are not instructions (`parallel` with a fixed form), bulleted steps in sequence | done |
| M14 | CLI | `check`, `fix`, `parse`, `packs` |

Validation: the gold derivations in `corpora/erg-gold/` (ERG `mrs` and `csli`
test suites) are the reference for M3–M9. The engine should reproduce the
gold tree among its analyses and reject the items marked ungrammatical.
