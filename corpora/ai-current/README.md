# Machine-written prose in the style of current chat models

Documents written for emdysi in the voice of chat-model output as of 2026:
the patterns current models produce that the 2023 samples of
`corpora/beemo` lack. Each document is a plausible answer to an ordinary
request (a blog post, a README section, a launch announcement, a support
reply, ...), written to be typical of the genre, not a parody: the
patterns appear at the density they do in real output.

- License: written for emdysi and covered by the repository's license
  (AGPL-3.0-or-later). No text was copied from other sources; names, products and
  figures are invented.
- Use: `rule_rates` on `samples.tsv` (made from the `.md` files by
  `to_tsv.py`) against the edited documents of `corpora/edited`, to find
  patterns the rule packs miss and to calibrate the ones they have
  (see docs/evaluation.md, "Machine-written prose from current models").
