# Anthropic HH-RLHF, helpful-base (sample)

- Upstream: https://github.com/anthropics/hh-rlhf,
  `helpful-base/test.jsonl.gz` (a Git LFS object), commit
  `c72f5cee8eb7b4d2ea5617657f4430d5e333af07` (2025-06-17); retrieved
  2026-10-02 from
  `https://media.githubusercontent.com/media/anthropics/hh-rlhf/c72f5cee8eb7b4d2ea5617657f4430d5e333af07/helpful-base/test.jsonl.gz`.
  SHA-256 `8be3fc1a13b27901631696f2be6f184c799f1baeaf145f53ac5db24960adc37b`,
  equal to the LFS pointer's oid (2,354 records). The README notes the
  GitHub repository is deprecated in favour of the Hugging Face copy, which
  holds the same data.
- License: MIT, "Copyright (c) 2022 Anthropic" (`LICENSE`, copied verbatim
  from the repository root).
- Attribution: Yuntao Bai et al. "Training a Helpful and Harmless
  Assistant with Reinforcement Learning from Human Feedback"
  (arXiv:2204.05862), Anthropic, 2022.
- Generator: Anthropic's context-distilled 52B-parameter language models
  (the "base" models of the paper), 2022; released by Anthropic under MIT.
- Provenance: crowdworkers conversed with the model and picked the
  preferred of two responses at each turn. Only the model's text is
  vendored.
- Row filter (`sample.py`): helpful-base only (no harmless-base,
  helpful-online, helpful-rejection-sampled or red-team data); test split;
  the first 200 records in file order whose final assistant turn is
  non-empty (here records 1 to 200).
- Column filter: from the `chosen` dialogue, only the text after the last
  `\n\nAssistant:` marker (the final assistant turn), with the record's
  1-based line number as id. The human turns (crowdworker text), earlier
  assistant turns and the `rejected` dialogue are dropped.
- Content note: the upstream README warns that the data "contain content
  that may be offensive or upsetting" (discriminatory language, abuse,
  violence, self-harm and more). This sample is from the helpful (not the
  harmless or red-team) data and a keyword scan found no such content, but
  it was not reviewed line by line, and model answers may be factually
  wrong.
- Format: TSV with a `#` header line; within cells, backslash, tab,
  newline and carriage return are escaped as `\\`, `\t`, `\n`, `\r`.
  Leading and trailing whitespace trimmed; no text otherwise changed.
- Use: tests and evaluation only (chat-assistant prose). Not used to train
  shipped data.
