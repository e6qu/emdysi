# Test corpora

Text used **only for tests and evaluation**: third-party corpora, and
samples written for this project. Each subdirectory
carries its own license and provenance note. Nothing here is compiled into the
published crates.

Rules for adding a corpus:

1. Check that the license permits redistribution (public domain, MIT/BSD/Apache, CC0,
   CC BY). CC BY-SA material may be added only in its own subdirectory with the
   license text, and must never be used to train shipped data.
   NonCommercial (NC) or no-derivatives (ND) material, LDC data and text of
   unclear copyright are not allowed.
2. Record the upstream URL, version/commit, license and a short provenance
   note in `<corpus>/SOURCE.md`, and copy the license text alongside.
3. Verify the license from the primary source, not a secondary listing.
4. Add an entry to [`VENDORED.toml`](../VENDORED.toml) (upstream, exact
   version, retrieval date, SPDX license, license file, copyright holders,
   provenance, modifications, use; for machine-generated text also
   `ai_generated = true` and `generator`, the models that wrote it and the
   terms on their output) and run
   `python3 scripts/check-vendored.py --update` to pin the files' hashes in
   `VENDORED.sha256`. CI fails on any file that is not registered or whose
   content changed. Where a dataset mixes material under different terms,
   vendor only the permitted rows and columns (for example, model output
   but not NC-licensed prompts), record the row and column filter in
   `modifications` and `SOURCE.md`, and keep the script that applies it
   (`sample.py`) beside the sample.

| Corpus | License | Contents |
|---|---|---|
| [`erg-gold/`](erg-gold/SOURCE.md) | MIT (ERG annotations); text by the ERG authors or public domain | ERG test suites `mrs`, `csli`, `esd`, `control`, `ccs` and the public-domain `sh-spec` (Sherlock Holmes) with gold derivations |
| [`golden-rules/`](golden-rules/SOURCE.md) | MIT | Sentence-segmentation test cases from pySBD / Pragmatic Segmenter |
| [`ai-prose/`](ai-prose/README.md) | MIT (written for this project) | Short documents in the style of machine-written prose, plus a plainly written control, with the expected diagnostics of the built-in packs |
| [`ai-treebank/`](ai-treebank/README.md) | MIT (annotations written for this project; Beemo sentences MIT) | 160 sentences of machine-written prose with the right ERG reading chosen by hand, for measuring parse ranking |
| [`blimp/`](blimp/SOURCE.md) | CC BY 4.0 | Sample of BLiMP (2,010 minimal pairs, 67 paradigms of syntax, morphology and semantics) |
| [`zorro/`](zorro/SOURCE.md) | MIT | Sample of Zorro (690 minimal pairs, 23 paradigms) |
| [`beemo/`](beemo/SOURCE.md) | MIT | Sample of Beemo (200 outputs of zephyr-7b-beta, Mistral-7B and Mixtral-8x7B with expert edits; No Robots prompts and human texts excluded) |
| [`cheat/`](cheat/SOURCE.md) | MIT | Sample of CHEAT (200 ChatGPT-written abstracts; IEEE titles, keywords and abstracts excluded) |
| [`hh-rlhf/`](hh-rlhf/SOURCE.md) | MIT | Sample of Anthropic HH-RLHF helpful-base (200 final assistant turns; human turns excluded) |
| [`edited/`](edited/SOURCE.md) | Public domain, CC0, MIT, CC BY 4.0 (one file each) | Edited text in several genres (a novel, US government guidance, the Rust book and blog, Kubernetes docs), for measuring false flags |
| [`edited-by-sa/`](edited-by-sa/SOURCE.md) | CC BY-SA 3.0 | 500 Wikipedia sentences from UD English PUD (news and encyclopedia), for measuring false flags |
| [`real-errors/`](real-errors/SOURCE.md) | MIT, CC BY 4.0 (one file each) | 377 paragraphs before and after typo, spelling and grammar fixes in the Rust book and the Kubernetes docs, for measuring how many real errors are caught |

## Considered and not vendored

- **LAMP** (Salesforce `creativity_eval`, Chakrabarty et al., CHI 2025):
  LLM-written paragraphs with professional writers' edits. The repository's
  LICENSE is BSD-3-Clause, but its `AI_ETHICS.md` says the release "is for
  research purposes only", which leaves the data's terms unclear (rule 1).
  Its writing instructions are derived from New Yorker paragraphs, and
  some rows come from Llama models whose licenses attach terms to outputs.
  Checked at commit `3d029879df6878f611363db88cc02d465699bc51`.
- **UltraFeedback** (MIT): the responses are on Hugging Face only, which
  could not be reached to verify them.
