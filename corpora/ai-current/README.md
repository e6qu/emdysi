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
  The first 24 documents (2026-10-17) were used to write the rules; the 12
  added later the same day (release notes, a billing FAQ, a git tutorial,
  a personal statement, a talk proposal, a product comparison, a grant
  summary, a postmortem, a book review, a social thread, a wedding toast,
  a course description) were first measured as unseen text, then used
  for more rules.
