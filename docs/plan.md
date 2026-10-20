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

1. **A real model (M20, must have).** Measured since 2026-10-19 with
   Gemma 4 E2B and E4B (4-bit GGUF, in-process; evaluation.md, "Grammar
   and spelling fixes with a real model"). What holds so far:
   - Asked for a letter, a small model answers by position; compared by
     likelihood, it picks the writer's correction of a real error 92% of
     the time (E4B 94%). Choices among versions now use likelihood.
   - Comparing versions needs a wide margin, or correct sentences are
     changed: plain log-probabilities at temperature 6 (since 2026-10-20;
     5 before) change 5 of 527 correct originals of real-error paragraphs
     and keep 18 of 31 corrections. Subtracting the words alone (SLOR) is
     far worse. In a sentence the grammar accepts no word is dropped
     unless doubled: the false changes were mostly dropped words.
   - Finding errors is the weak part, for both models alike: a word's
     surprise, or the model's own next-word expectations, locate the
     corrected word about 30% of the time and barely separate wrong
     sentences from right ones (AUC 0.60). Without a locator the right
     edit is mostly past the candidates parsed (120) or the versions
     shown (27): 92 of 197 grammar errors (`NOFOCUS=1 fix_bound`).
   - Where a model can gain (the 34 pairs of a quarter that a perfect
     model catches and the grammar does not), E2B with every sentence
     screened in caught 3, all wrong inflections. When it saw the
     writer's correction it mostly chose it, but with too little margin
     to tell it from changes to correct sentences; E4B is no better there.

   So with a 2–4 GB model on a CPU, grammar plus model adds little beyond
   the grammar. Still to do: false flags with the model on the
   development sets, to set the defaults (the likelihood temperature, the
   screen) so that the model never costs precision; then a much larger
   model, served from a machine with more memory, on the same saved
   questions (`LOG.versions.tsv`, `examples/versions`) to see whether
   the design pays off with a stronger one. Until then priority 2 is
   where recall can still be gained without one.

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
   another width for the first beam, pruning lexical entries by a
   statistical tagger (gold analyses lost, no time saved) and keeping a
   rule's structure loaded in the unifier between attempts (same
   readings, no time saved). Exact-size copies in the unifier saved 6%
   (2026-10-19). What is left
   is the cost of searches that do find the strict analysis with a wide
   beam; making them cheaper means a faster parser core, not a cleverer
   cascade: a structure-sharing unifier, days of work for an estimated
   5–10%. A model adds candidate verification on top (up to 120
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

7. **Parse ranking.** The shipped averaged perceptron is 81.7% exact
   match held out and right for 57 of 97 judged machine-written
   sentences. Log-linear (84.8% held out) and head-word features (84.3%)
   score better on the gold profiles but worse on the judged sentences
   (48 and 35), and chart pruning guided by the ranker loses the right
   reading for 12 of 97 with other models (evaluation.md). A fair
   comparison needs readings judged independently of any one ranker,
   from several rankers' top choices, with pruning that does not depend
   on the model being evaluated; that judging is slow and wants a
   person's spot checks.

Status on 2026-10-19: apart from the model evaluation, every item above
is either bounded by the model (126 of the 172 missed grammar errors
parse as correct English) or a multi-day effort with an uncertain gain
(the unifier rewrite, independent ranking judgements). Both development
sets have been mined for coverage gaps. Start a new session with the
model evaluation.

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
| M15 | Decision engine: pluggable local models, typed decisions with calibration, `en decide`, `decisions` pack | done; versions of a sentence compared by likelihood, words by surprise (2026-10-19) |
| M16 | Ambiguity settled by the document's own phrases and by the model | done; best reading right for 58 of 97 judged machine-written sentences |
| M17 | Precise error claims: false-flag corpus in five genres, proved parse failure, verified named errors | done; see the table above |
| M18 | Grammar extensions in emdysi's own TDL; stress sentences; ambiguity reported | done |
| M19 | Coverage of edited text | ongoing; priority 4 |
| M20 | Small local model with the grammar: verified corrections chosen by the model, spelling choices, readings, AI-prose judgements | measured with Gemma 4 E2B and E4B: choosing works, finding errors does not yet; priority 1 |
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
3. **Running the model evaluation.** First check that the session can
   reach the model host: `curl -sS -o /dev/null -w '%{http_code}\n'
   https://huggingface.co` should print 200, not a `CONNECT tunnel
   failed, response 403`; if not, the environment's network access needs
   huggingface.co (and the download hosts it redirects to). The models
   used: `ggml-org/gemma-4-E2B-it-GGUF` and `ggml-org/gemma-4-E4B-it-GGUF`,
   `*-Q4_0.gguf` (2.8 and 4.6 GB). Build with `cargo build --release
   --features llama --examples --bins` (`--examples` alone leaves out
   `en`). A model run takes hours, so measure on a sample and watch it:
   - `real_errors` with `SHARD=0/4` (94 pairs), `THREADS=2
     EMDYSI_LM_THREADS=2` (two pairs at once, each model on two cores)
     and `LOG=FILE`; it journals each pair, so an interrupted run
     resumes. Run the same shard without a model and with `ORACLE=1`
     once, as baselines.
   - `scripts/model-eval/status.sh OUT.tsv ERR LOG` (with `BASE=DIR` of
     the baselines) prints the pairs done, recall against the baselines,
     and how the model answers each kind of question.
   - The versions questions go to `LOG.versions.tsv`; `examples/versions`
     scores them once (`SCORES.tsv`) and `scripts/model-eval/margins.py`
     tries ways of choosing and margins on the scores without the model.
   - `false_flags` with `SHARD=0/4` (55 documents, 673 sentences of the
     first development set; 0 flags without a model).
   - `examples/surprisal` measures locating and screening on the minimal
     pairs of `scripts/model-eval/questions.py`.

   Two processes with models at once on four cores run at a fifth of the
   speed each: pause one (`kill -STOP`, `kill -CONT`) rather than share.
