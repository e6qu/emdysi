# Plan

The work follows the pipeline in [prior-art.md](prior-art.md#proposed-architecture-first-cut),
with the ERG port (decision D4) as the parsing core.

| Milestone | Scope | Status |
|---|---|---|
| M0 | Workspace, decisions, license policy, vendored ERG, test corpora | done |
| M1 | `emdysi-tdl`: TDL reader that loads the whole ERG | done |
| M2 | Type hierarchy: multiple inheritance, addenda, GLB closure, feature appropriateness | done |
| M3 | Typed feature structures: unification, copying, subsumption; well-typed expansion of every ERG type, rule and lexical entry | done |
| M4 | REPP tokenizer (`rpp/`) and token lattice with character spans | REPP done |
| M5 | Chart mapping: token-mapping rules (`tmr/`) and lexical filtering (`lfr.tdl`) | done |
| M6 | Orthographic morphology (`%suffix`/`%prefix`, letter sets, `irregs.tab`) and lexical lookup, including generic entries for unknown words | done (multiword entries inflect on the last word) |
| M7 | Rule-based POS tagger feeding `+TNT` tags (decision D3) | first heuristic version |
| M8 | Chart parser: agenda, rule filter, quick-check, ambiguity packing, unpacking, root conditions | agenda (shorter spans first), quick-check (incl. two-step for binary rules), packing, unpacking, root conditions done; chart pruning for inputs over 20 positions (the ranker's local features score edges, 40 kept per span) and best-first unpacking; packing compares edges in place, packed and frozen edges release their structures, and a binary rule shares its unification with one daughter across all candidates for the other (checkpoint and rollback); a node budget (25 million nodes, about 1 GB) counts only the structures the chart keeps |
| M9a | Semantics: MRS read-out (VPM, SimpleMRS, isomorphism, well-formedness checks) | done: identical to the gold MRS for all 2,966 gold analyses reproduced (mrs, csli, esd, ccs, control, sh-spec) |
| M9b | Named grammatical errors with the ERG's mal-rule variant (`educ/`) | done: on csli, 104 of 388 ungrammatical items get a named error and 70 a generic one; 17 of 965 grammatical items get a false named error |
| M9 | Output: derivation trees, labelled phrase-structure trees, sentence type; plain-text and Markdown rendering | derivations and labelled trees done |
| M9d | Semantic prose checks from the MRS: missing comparand, agentless passive, stacked negation, bare demonstrative, tense shift | done (`semantics` rule kind) |
| M9e | Guarded rewriting and typo correction (decision D10): candidates from fixes, spelling and an optional local model (llama.cpp, GGUF), kept only when the grammar accepts them, the MRS content is preserved and no new diagnostic appears; `en rewrite` | done; the model path is tested with a scripted stand-in (no weights can be downloaded in this environment) |
| M9c | Minimal-pair suites (BLiMP and Zorro samples) as grammaticality tests | done: see [evaluation.md](evaluation.md); morphosyntax 87-100% of pairs, semantic/pragmatic paradigms left to the statistical and model layers |
| M10 | Parse selection and robust fallback for fragments | averaged-perceptron ranker on clean gold: 81.7% exact match held out on 2,626 items of all lengths (77.9% when trained on the earlier, short-only set); the ERG's fragment/informal roots act as fallback, then a cover of the input by the fewest partial analyses |
| M11 | Sentence segmentation and Markdown input with span mapping | done (46/47 Golden Rules) |
| M12 | Spelling: SCOWL-based lexicon, suggestions, confident auto-fix | done |
| M13 | Rules engine and DSL; AI-prose and style-guide packs | 28 rule kinds, 6 default and 7 opt-in packs; rule scopes (heading, lead, body, ...) and per-rule examples run as tests; word rules match lemmas and inflect their fixes (*utilizes* to *uses*) |
| M13b | Document structure and terminology (research: `reports/AI corpora and style guide rules.md`, not in the repository): heading hierarchy, bottom line up front, paragraph and section size, parallel headings and lists; acronym definitions (Schwartz–Hearst), a TBX-style glossary, one spelling per term, coined words and concept names, hyphen chains, -ly hyphens, noun stacks | done (`structure` and `terms` packs) |
| M13c | Rule data imported from other linters under permissive licenses: Vale's Microsoft, Google and Elastic packages and the words/* lists, converted by `scripts/import-rules.py` into opt-in packs (`existence` and `substitution` kinds) | done; proselint, Red Hat, GitLab and LanguageTool data excluded for licensing reasons (see rules.md) |
| M13d | Opening paragraphs and procedures: `vague-lead` (no concrete claim in the semantics of the opening), numbered steps that are not instructions (`parallel` with a fixed form), bulleted steps in sequence | done |
| M14 | CLI | `check`, `fix`, `parse`, `rewrite`, `decide`, `glossary`, `packs` |
| M15 | Decision engine (decision D12): pluggable local models (`emdysi-lm`: GGUF via llama.cpp, OpenAI-compatible servers, scripted stand-in), typed-probability decisions with debiasing and calibration, `en decide`, `decide` rules (`decisions` pack), parse disambiguation, rewrite choice; a tiny CPU-trained test model in GGUF and MLX | done; tested end to end against llama.cpp in-process, `llama-server` and `mlx_lm.server` with the tiny model. Next: real models (Gemma 4 E2B/E4B) once they can be downloaded, measured with `--decide-readings` on the AI-prose treebank |
| M16 | Ambiguity settled by context: a hand-judged treebank of machine-written sentences (`corpora/ai-treebank`), citations masked before parsing, readings settled by phrases the rest of the document uses (on by default), the model asked with the surrounding sentences | done; the best reading is right for 57 of 97 judged sentences in their documents, 58 with the document's phrases (no regressions). Next: more judged sentences, with right readings judged independently of the shipped ranker (see evaluation.md: better held-out rankers, log-linear or with head-word features, lose on AI prose, mostly through chart pruning); most errors are rule choices the document cannot inform (fronted adverbs, unknown participles) |
| M17 | Precise error claims (decision D13): edited-text corpus in five genres with a false-flag evaluation; proved parse failure, shared and verified named errors, typo evidence, larger accepted vocabulary, code-like tokens masked; `core.grammar` moved to the opt-in `coverage` pack | first development set: flags 374 -> 2.0 per 1,000 sentences of edited text; second development set: 6.4 per 1,000; held-out test set (8,371 sentences, never inspected): 7.9 per 1,000, spelling the largest part. Recall on 377 real errors from documentation histories: 162. Next: more recall without false flags (agreement with irregular verbs, *was*/*were*, is now caught), spelling evidence beyond one edit, more genres |
| M18 | Grammar extensions and ambiguity (decision D14): emdysi's own TDL files for constructions and words the ERG lacks (comparative correlatives, sentence-initial conjunctions, contractions as formal, formal counterparts of informal rules for fronted adjuncts, comma-separated adjectives and US dates, lexical entries); stress sentences; readings grouped by meaning with calibrated probabilities | done: every stress sentence gets a full analysis and no error claim; the famous ambiguous ones are reported as ambiguous |
| M19 | Coverage of edited text: sentences without a full analysis found with `culprits`, fixed in the grammar or the search | 300-sentence samples (checker segmentation): 9 of 300 without a strict reading on the first development set, 18 on the second; a fresh 948-sentence sample: 54 -> 45 with mentions and *provision*, then the lexical gaps of 2026-10-13 (*compile time*, *redirect*, ...; see evaluation.md). Grammar added: gapping, subjects before paired commas, amounts like *£2bn*, mentions as one token. Next: the recurring gaps left (locative inversion, verbless *Many thanks to X*, extraposed *than* clauses, bare names before nouns such as *Longbourn estate*), found by scanning the corpora for words used in a part of speech the lexicon lacks (`noungaps.py`-style scans, see evaluation.md) |
| M20 | **Must have.** A small local classifier in the manner of Jev (decision D12): a 2 GB model (Gemma 4 E2B, 4-bit GGUF, or similar) used with the deterministic layers so that together they are more accurate than a much larger model alone. The grammar proposes and verifies, the model chooses in context: grammar corrections of sentences without a full analysis (`core.grammar-fix`: *The quick brown fox jump over the crazy wall.*), spelling fixes among several one-edit candidates (*chck*), close calls between readings (`--decide-readings`), the AI-prose judgements of the `decisions` pack. Used whenever a model is given (`--model`, `--server`, `--lm`) or configured once (`EMDYSI_LM`) | machinery done and tested with stand-in models (`crates/emdysi-check/tests/assist.rs`): candidate corrections (doubled words, other forms of a word, function-word substitution, deletion and insertion, ordered by where the analysis breaks down), verified by the strict grammar and chosen by the model in rounds; sentences the grammar accepts are screened by the model first; spelling choices among one-edit candidates; `EMDYSI_LM`. Upper bound with a perfect model (`real_errors` with `ORACLE=1`): 163 of 377 real errors without a model, 201 of 377 (53%) with it after the wider candidates of 2026-10-14, 253 of 377 (67%; grammar errors 107 of 199) with real-word typo candidates (*Manger*/*Manager*) and the model asked which word is wrong before the edits are tried, 286 of 377 (76%; grammar errors 137 of 199) with corrections verified as the checker parses them (within 30 seconds a sentence) and more candidates. `fix_bound` shows where each grammar error's correction is lost: 99 of 199 reachable (24 before). 121 of 199 once corrections are verified as the checker parses them (masked, with time by length), 128 with more candidates; the 30 still rejected are mostly long sentences whose clauses each parse (search, M8) and text that is not a sentence. Next here: corrections of two or more words. Next on a machine that can download it: Gemma 4 E2B with `scripts/model-eval/run.sh` |
| M21 | AI tropes and vacuous text: the tropes of machine-written prose (setups, engagement questions, mechanical openers, inflated words, both-sides framing, the not-about frame across sentences, rhetorical questions, one-line paragraphs) and their density per document (`ai-tells.dense`); filler, platitudes, and, from the semantics, sentences of abstractions, maxims and restatements (`substance`); new rule kinds `sentence-pair` and `tell-density` and the `one-line-paragraphs` check | done; word lists chosen by rates in machine-written against edited text; calibration on the first development set and the machine-written samples in evaluation.md. Next: more machine-written text from current models (the vendored samples are from 2023 models and lack the newer tropes), and the `decisions` pack with the M20 model for the judgements no pattern can make (explaining the obvious, fake specificity) |

Validation: the gold derivations in `corpora/erg-gold/` (ERG `mrs` and `csli`
test suites) are the reference for M3–M9. The engine should reproduce the
gold tree among its analyses and reject the items marked ungrammatical.

## Picking up

Where the work stands, and how to resume it on another machine.

1. **Build and test.** `cargo build --release --examples` then
   `cargo test --release --workspace` (the tests load the grammar several
   times; with less than 16 GB of memory use `-- --test-threads=1`). The
   grammar cache lives outside the repository (see the README).
2. **The M20 model (must have).** Download Gemma 4 E2B instruction-tuned,
   4-bit GGUF (about 2 GB), build with `--features llama` (or serve it with
   `llama-server` and use `--server`), and set
   `EMDYSI_LM=gguf:/path/to/model.gguf`. Then:
   - calibrate: `en decide --eval FILE.tsv` on questions with known answers
     (the tiny-model script writes such files; make one from the real-error
     corpus: original against corrected sentence), and pass the fitted
     `--temperature`;
   - measure recall with and without the model:
     `cargo run --release -p emdysi-check --example real_errors -- OUT.tsv`
     (163 of 377 without a model, 2026-10-13);
   - measure false flags with and without it: `THREADS=4 ... --example
     false_flags -- OUT.tsv` and `SET=dev2` (6 and 79 flags without a
     model); with an in-process GGUF model use `THREADS=1` (one model per
     thread);
   - compare with a much larger model served over the same API on the same
     questions; the aim is that grammar plus small model beats the large
     model alone.
   - tune `threshold` of `core.grammar-fix` and `model_threshold` of
     `core.spelling` (0.8 now) on these numbers.

   A model that doubts many sentences makes checking slower: each doubted
   sentence parses up to 120 candidate corrections (`MAX_CANDIDATES` in
   `crates/emdysi-check/src/assist.rs`), fewer once 27 of them parse.
   Measure the time with the model; if it is too slow, order the
   candidates by the model before parsing them (ask it among unverified
   versions first, then verify only its top choices), or parse them in
   parallel (the parse time limit is wall-clock, so not inside runs that
   already use every core). `SHARD=i/n` splits a `real_errors` run into
   parts; `fix_bound` (no model) shows which stage loses each grammar
   error's correction, and should be run after any change to the
   candidates.

   `scripts/model-eval/run.sh SPEC` runs all of this in order:
   calibration on half of the real-error questions
   (`scripts/model-eval/questions.py`, 377 questions: the sentence as
   written against the writer's correction) and measurement on the other
   half, recall with the grammar alone, with a perfect model (`ORACLE=1`,
   the upper bound for any model on the corrections the grammar verifies)
   and with the model, and false flags on both development sets.
3. **Measurement recipe for grammar changes** (every PR so far): coverage
   on the two 300-sentence samples and the 948-sentence sample (sampling
   script in evaluation.md, "Coverage, measured again"), the six gold
   profiles (`--example eval`), `real_errors`, and `false_flags` on both
   development sets; flags must not rise and gold must not change except
   for the better. Run them one at a time: two parses at once can exhaust
   memory.
4. **Open items**: M19's remaining gaps; M21's calibration on newer
   machine-written text; speed on the slowest ordinary sentences (6–8 s for
   17-word sentences with a strict reading, e.g. *It's better to take the
   time to rewrite to avoid needing to define a bunch of terms.*);
   *Consistency builds trust.* is ranked as a compound noun (*consistency
   builds*) with *trust* as the verb; the checker reports it as ambiguous
   (57% against 39%), which is the close call `--decide-readings` puts to
   the model, so it is measured with the model. For M20 without a model:
   the corrected sentences the grammar rejects (`fix_bound`'s "corrected
   sentence not strict", 30: 15 long sentences whose clauses parse, which
   wait on the search, M8; 8 that are not sentences; see evaluation.md,
   "The corrections the grammar rejected"). With the model: how often it names the
   wrong word (`LOCATE_QUESTION`); the bound of 286 assumes it always
   names the right one, and `NOFOCUS=1 fix_bound` gives the bound without
   that question (53 of 199 grammar errors reachable, against 99).

