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
| Real errors caught (377, from documentation histories) | 183 (49%): spelling 159 of 178, grammar 24 of 199 | 167 before the spelling work of 2026-10-17 (this branch) |
| ... with a perfect model choosing among verified corrections | 337 (89%): grammar 165 of 199 | the upper bound for any model; no real model measured yet |
| False flags on edited text, per 1,000 sentences | 1.6 (first development set), 3.7 (second), 3.5 (held out, 8,371 sentences) | most of the second set's flags point at real errors or real spelling mixes in the source |
| Gold analyses (ERG test suites) | MRS identical for all 2,966 reproduced; ranker 81.7% exact match held out | |
| Speed | about 2.4 s of CPU per sentence of edited text (cold parse cache) | held-out set: 115 minutes on 3 threads |

## Priorities

1. **A real model (M20, must have).** Everything model-assisted is built
   and tested with stand-ins; the 337 above assumes a model that always
   chooses right and always names the wrong word when asked. The open
   question of the whole design is what a 2 GB model (Gemma 4 E2B, 4-bit
   GGUF, or similar) actually gets, and whether grammar plus small model
   beats a much larger model alone. Blocked in the cloud environment so
   far: its network policy denies huggingface.co. See "Running the model
   evaluation" below.
2. **Grammar errors without a model.** 24 of 199 is the weakest number in
   the table. The mal-rule variant names errors only when a complete
   search finds nothing else; most real grammar errors (missing function
   words, wrong forms) leave the sentence parseable by some informal
   reading or cut off by the search. What a model-free path could reach
   is bounded by `fix_bound` (142 of 199 corrections verifiable); the
   problem is choosing among the verified ones without a model.
3. **Speed.** 2.4 s of CPU per sentence is minutes for a long document
   and too slow for an editor. Where the time goes (parse, the
   grammar-error variant, verification of corrections) needs profiling on
   ordinary edited text before any optimization; a model adds candidate
   verification on top (up to 120 parses for a doubted sentence).
4. **Coverage of edited text (M19).** About 4% of edited sentences get no
   strict reading (40 of a 948-sentence sample, mostly archaic or not
   prose). Remaining recurring gaps: locative inversion, verbless *Many
   thanks to X*, extraposed *than* clauses, bare names before nouns
   (*Longbourn estate*).
5. **Machine-written prose (M21).** The vendored samples are from 2023
   models and lack newer tropes; the judgements no pattern can make
   (explaining the obvious, fake specificity) wait on the model.

Decided against, with measurements in evaluation.md: work limits in
place of the parser's time limits (no work measure tracks time well
enough), reporting a single verified correction without a model, and
typos-dict as a misspelling list (share-alike provenance).

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
