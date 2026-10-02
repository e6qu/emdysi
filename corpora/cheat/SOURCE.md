# CHEAT: ChatGPT-written abstracts (sample)

- Upstream: https://github.com/botianzhe/CHEAT,
  `data/ieee-chatgpt-generation.jsonl`, commit
  `99bdfc2842963a5f3ba8f0a8439b0e1c6df82059` (2025-04-26); retrieved
  2026-10-02. SHA-256 of the upstream file:
  `9d5606c9445cad4faa8e15e84d081df86ce62623549017e5f5ea2aced1050909`
  (15,395 records).
- License: MIT, "Copyright (c) 2023 security" (`LICENSE`, copied verbatim
  from the repository root). The README states no other terms.
- Attribution: the paper "CHEAT: A Large-scale Dataset for Detecting
  CHatGPT-writtEn AbsTracts" (upstream README).
- Generator: OpenAI ChatGPT, 2023 (the README does not name the model
  version). OpenAI's terms of use assign to the user any rights in the
  output; the dataset authors released it under MIT.
- Provenance: per the upstream README, the `generation` file is "the first
  pass generation by ChatGPT": an abstract written by ChatGPT given the
  title and keywords of an IEEE paper. The abstract text is model output,
  not IEEE text.
- Row filter (`sample.py`): the first 200 records in file order (ids
  8600003 onward).
- Column filter: kept `id` and `abstract` (the generated text). Dropped
  `title` and `keyword`, which are copied from the IEEE papers.
- Excluded files: `ieee-init.*` (the original IEEE abstracts, IEEE
  copyright), `ieee-chatgpt-polish.*` and `ieee-chatgpt-fusion.*`
  (ChatGPT rewrites of, or hybrids with, those abstracts, hence derivative
  of IEEE-copyrighted text), and the `.xlsx` copies.
- Format: TSV with a `#` header line; within cells, backslash, tab,
  newline and carriage return are escaped as `\\`, `\t`, `\n`, `\r`.
  Leading and trailing whitespace trimmed; no text otherwise changed.
- Use: tests and evaluation only (machine-generated academic prose). Not
  used to train shipped data.
