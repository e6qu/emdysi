# Beemo: Benchmark of Expert-edited Machine-generated Outputs (sample)

- Upstream: https://github.com/Toloka/beemo, `dataset.parquet`, commit
  `cca3b817eb6d0bf7a60f885f3022696553eabb63` (2025-02-04); retrieved
  2026-10-02. SHA-256 of the upstream `dataset.parquet`:
  `5df88f7941e873db3c6e2be4e4085db50fa824f7fc626685f0e317e935a188db`
  (2,187 rows).
- License: MIT, "Copyright (c) 2024 Toloka" (`LICENSE`, copied verbatim
  from the repository root). The upstream README's License section is
  narrower than the LICENSE file and is followed here:
  - "The prompts and human-written texts from No Robots are under the
    original dataset's license: CC-BY-NC-4.0." These are **not** vendored.
  - "The machine-generated texts and their LLM-edited versions are subject
    to the underlying instruction-finetuned LLMs' licensing terms
    mentioned in Table 1."
  - "The expert-edited machine-generated texts are available under the MIT
    license, unless otherwise specified in the underlying
    instruction-finetuned LLMs' licensing terms."
- Copyright and attribution: Toloka. Ekaterina Artemova, Jason Lucas,
  Saranya Venkatraman, Jooyoung Lee, Sergei Tilga, Adaku Uchendu and
  Vladislav Mikhailov. "Beemo: Benchmark of Expert-edited
  Machine-generated Outputs." NAACL 2025 (arXiv:2411.04032).
- Generators (licenses as listed in upstream Table 1 and the model cards):
  `HuggingFaceH4/zephyr-7b-beta` (MIT),
  `mistralai/Mistral-7B-Instruct-v0.1` (Apache-2.0) and
  `mistralai/Mixtral-8x7B-Instruct-v0.1` (Apache-2.0). Expert edits by
  Toloka's in-house annotators (MIT).
- Provenance: `model_output` is the raw output of the generator for a
  No Robots prompt; `expert_edited` (upstream `human_edits`) is that output
  edited by an expert annotator.
- Row filter (`sample.py`), in upstream order of `id`:
  1. `model` is one of the three generators above. Rows from Llama 2
     (Llama 2 Community License), Tulu 2 (AI2 ImpACT) and Gemma (Gemma
     Terms of Use) are excluded, since those terms attach to the outputs
     and are not on the allowlist in `corpora/README.md`.
  2. `category` is `Generation` or `Open QA`. `Rewrite`, `Summarize` and
     `Closed QA` rows are excluded: their prompts embed a No Robots input
     text (CC BY-NC 4.0) that the output restates or quotes, so the output
     may carry NC-licensed text.
  3. The first 200 such rows (upstream ids 214 to 859): 174 Generation
     and 26 Open QA; by generator zephyr-7b-beta 72, Mistral-7B 54,
     Mixtral-8x7B 74.
- Column filter: kept `id`, `model`, `category`, `model_output`,
  `human_edits` (as `expert_edited`). Dropped `prompt_id`, `prompt` and
  `human_output` (No Robots, CC BY-NC 4.0), and `llama-3.1-70b_edits` and
  `gpt-4o_edits` (subject to the Llama 3.1 and OpenAI terms).
- Format: TSV with a `#` header line; within cells, backslash, tab,
  newline and carriage return are escaped as `\\`, `\t`, `\n`, `\r`. No
  text was otherwise changed.
- Use: tests and evaluation only (machine-generated prose with an
  expert-edited counterpart). Not used to train shipped data.
