# Plan

emdysi checks English prose with a port of the English Resource Grammar
(decision D4) at its core, rule packs for style and machine-written text
on top, and a small local model for the choices the grammar cannot make
(decision D12). The architecture is in
[prior-art.md](prior-art.md#proposed-architecture-first-cut); the
measurements behind every number below are in
[evaluation.md](evaluation.md).

## Where it stands

The pipeline is built end to end: tokenization, morphology, chart parsing,
parse ranking, semantics (MRS), named grammar errors, spelling, 28 rule
kinds in 13 packs, Markdown input, the CLI (`check`, `fix`, `parse`,
`rewrite`, `decide`, `glossary`, `packs`) and the model interface (GGUF
in-process, OpenAI-compatible servers). Headline numbers, without a model
unless stated:

| Measure | Now | Notes |
|---|---|---|
| Real errors caught (374, from documentation histories) | 192 (51%): spelling 167 of 177, grammar 25 of 197 | 167 before the spelling work of 2026-10-17, 183 before codespell's list; 377 pairs until three that were all front matter left (2026-10-17) |
| ... with a perfect model choosing among verified corrections | 337 of 377 (89%): grammar 165 of 199 | the upper bound for any model, measured before the corpus repair; no real model measured yet |
| False flags on edited text, per 1,000 sentences | 1.6 (first development set, 4 flags), 4.8 (second, 64), 3.7 (held out, 31 in 8,371 sentences) | most of the second set's flags point at real errors or real spelling mixes in the source (`docs/evaluation.md`) |
| Gold analyses (ERG test suites) | MRS identical for all 2,966 reproduced; ranker 81.7% exact match held out | |
| Speed | about 1.3 s of parsing per sentence of edited text (held-out set, cold parse cache, 2026-10-18), plus 0.25 s for the grammar-error variant | 1.7 s of CPU per sentence before giving up after an exhausted search, 2.1 s before the grammar-error variant was gated (2026-10-17); parsing is most of the time |
| Rule data | 2,580 LanguageTool rules (579 of them checked against the parse's parts of speech), 98 slop rules, 26 GitLab rules (opt-in packs); codespell, CMUdict, UniMorph, Open English WordNet, UD English EWT vendored | emdysi is AGPL-3.0-or-later since 2026-10-17 (decision D15), so share-alike and GPL data can be used |

## Priorities

1. **A real model (M20, must have).** Everything model-assisted is built
   and tested with stand-ins; the 337 above assumes a model that always
   chooses right and always names the wrong word when asked. The open
   question of the whole design is what a 2 GB model (Gemma 4 E2B, 4-bit
   GGUF, or similar) actually gets, and whether grammar plus small model
   beats a much larger model alone. Blocked in the cloud environment so
   far: its network policy denies huggingface.co. See "Running the model
   evaluation" below.
2. **Grammar errors without a model.** 25 of 197 is the weakest number in
   the table. The mal-rule variant names errors only when a complete
   search finds fragments only. Of the 172 real grammar errors missed
   (2026-10-18), 126 leave a strict reading (a wrong word that is still
   English, or a legal but unlikely analysis: *you wants* as *you* plus a
   plural noun, *A quota can either limits* with *quota can* a compound,
   a capitalized first word read as a name), 19 a search that does not
   finish, 14 a finished search where the variant names nothing, and 9
   readings under the ERG's informal, robust or fragment roots (widening
   the diagnosis to them caught none; see evaluation.md). What a
   model-free path could reach is bounded by `fix_bound` (142 of 199
   corrections verifiable, before the corpus repair); the problem is
   choosing among the verified ones without a model.
3. **Speed.** 1.3 s of parsing per sentence is still minutes for a long
   document. Profiled (`timing`): the parser takes 82%, rules almost
   nothing; the 4% of sentences that reach the time limit take 31% of
   parse time and yield almost no strict readings. Next: make those
   sentences cheaper. Done (2026-10-18): the searches after one that ran out
   without a strict analysis are no longer tried, 15% less parsing time
   on the held-out set, no strict analysis lost (evaluation.md).
   Measured and rejected since: fewer searches for long sentences,
   another width for the first beam, and pruning lexical entries by a
   statistical tagger (gold analyses lost, no time saved). What is left
   is the cost of searches that do find the strict analysis with a wide
   beam; making them cheaper means a faster parser core, not a cleverer
   cascade. A model adds candidate verification on top (up to 120
   parses for a doubted sentence).
4. **Coverage of edited text (M19).** About 4% of edited sentences get no
   strict reading (40 of a 948-sentence sample, mostly archaic or not
   prose). Locative inversion, verbless *Many thanks to X*, bare names
   before nouns (*Longbourn estate*) and a *than* phrase after a
   modifier now parse. Remaining recurring gaps (evaluation.md, second
   development set): *X, Y, or both* after verbs or participles, *than*
   with a gapped clause (*more CPU than they request*), *short for* with
   an adjective, and documentation shorthand without a subject or
   determiner. Both development sets have now been mined for gaps; the
   held-out set is kept for measuring.
5. **Machine-written prose (M21).** 24 samples of current-model prose
   (`corpora/ai-current`) found the newer tropes (bold lead-ins, emoji
   headings, reader hooks), and the `slop` pack adds bheijden/slop's
   tested patterns. The judgements no pattern can make (explaining the
   obvious, fake specificity) wait on the model.
6. **More LanguageTool rules.** 579 rules now check parts of speech
   against the parse, and suggestions inflect, change case and rewrite
   matched words; about 600 more need LanguageTool's chunker. None of
   the converted rules catches the real grammar errors (24 of 199 with
   the pack or without, when measured): those need the grammar.

Decided against, with measurements in evaluation.md: work limits in
place of the parser's time limits (no work measure tracks time well
enough), reporting a single verified correction without a model, and
LanguageTool's wordiness and redundancy lists (wrong replacements, off by
default in LanguageTool too).

## Milestones

| Milestone | Scope | Status |
|---|---|---|
| M0–M6 | Workspace, license policy, vendored ERG; TDL reader; types and feature structures; REPP; chart mapping; morphology and lexical lookup | done |
| M7 | Rule-based POS tagger feeding `+TNT` tags (decision D3) | first heuristic version |
| M8 | Chart parser: packing, unpacking, root conditions, chart pruning (40 edges a span over 20 positions), best-first unpacking, wider searches when no strict reading is found | done; speed is priority 3 |
| M9, M9a–e | Trees and output; MRS (identical to gold); named errors from the mal-rule variant; minimal pairs (BLiMP, Zorro); semantic prose checks; guarded rewriting (`en rewrite`) | done |
| M10 | Parse ranking (averaged perceptron, 81.7% exact match held out); fragments and partial covers as fallback | done |
| M11–M14 | Segmentation and Markdown; spelling; rules engine and packs (structure, terminology, imported Vale rules, openings and procedures); CLI | done |
| M15 | Decision engine: pluggable local models, typed decisions with calibration, `en decide`, `decisions` pack | done with a tiny test model; real model is priority 1 |
| M16 | Ambiguity settled by the document's own phrases and by the model | done; best reading right for 58 of 97 judged machine-written sentences |
| M17 | Precise error claims: false-flag corpus in five genres, proved parse failure, verified named errors | done; see the table above |
| M18 | Grammar extensions in emdysi's own TDL; stress sentences; ambiguity reported | done |
| M19 | Coverage of edited text | ongoing; priority 4 |
| M20 | Small local model with the grammar: verified corrections chosen by the model, spelling choices, readings, AI-prose judgements | machinery done; real model is priority 1 |
| M21 | Machine-written tropes and vacuous text | done; newer samples are priority 5 |

## Picking up

1. **Build and test.** `cargo build --release --examples`, then
   `cargo test --release --workspace` (the tests load the grammar several
   times; with less than 16 GB of memory add `-- --test-threads=1`). The
   grammar cache lives outside the repository (see the README).
2. **Measuring a change.** `real_errors` (recall; `ORACLE=1` for the
   perfect-model bound, `SHARD=i/n` to split it, `ONLY=a|b` for a few
   pairs) and `false_flags` (first development set; `SET=dev2`,
   `SET=heldout`), both in `crates/emdysi-check/examples`. A grammar
   change adds coverage on the 300- and 948-sentence samples and the gold
   profiles (`--example eval`). Flags must not rise and gold must not
   change except for the better. Long runs journal their progress and
   resume; a perfect-model run varies by a few pairs between runs (wall-clock
   time limits), so compare runs on a warm parse cache; at most two
   measurement processes at once (memory). A spelling change can be
   checked in minutes with `en check --no-parse` over the corpora's
   documents against the main build.
3. **Running the model evaluation.** Download Gemma 4 E2B
   instruction-tuned, 4-bit GGUF (about 2 GB); build with
   `--features llama` or serve it with `llama-server`; set
   `EMDYSI_LM=gguf:/path/to/model.gguf` (or `http://HOST:PORT`). Then
   `scripts/model-eval/run.sh SPEC` calibrates on half of the real-error
   questions and measures on the other half: recall with the grammar
   alone, with a perfect model and with the model, and false flags on
   both development sets. With an in-process model use `THREADS=1` (one
   model per thread). Then compare with a much larger model on the same
   questions, and tune `threshold` of `core.grammar-fix` and
   `model_threshold` of `core.spelling` (0.8) on the results. How often
   the model names the wrong word when asked where a sentence goes wrong
   matters most: without that question only 53 of 199 grammar errors are
   reachable (`NOFOCUS=1 fix_bound`), against 99 with it.
