# retext-equality word lists (English)

- Upstream: https://github.com/retextjs/retext-equality, commit
  `192deddf13bc2823540b0d9a0a6e17fb6d995bcd` (2024-05-30); retrieved
  2026-10-02. `data/en/*.yml` (as `en/`) and `license` are copied verbatim.
- License: MIT, "Copyright (c) 2015 Titus Wormer" (`license`). This is the
  data behind the alex linter.
- Provenance: entries written by the project's contributors; some carry a
  `source` link to a published style guide (for example the National
  Center on Disability and Journalism) that the short `note` paraphrases.
  No list is copied from a third party.
- Use: `scripts/import-rules.py` converts the 253 `type: basic` entries
  (an inconsiderate phrase and considerate alternatives) into the opt-in
  pack `packs/equality.toml`, one rule per category file. The 172
  `type: or` entries (pairs such as *he or she*, flagged only when one side
  is missing) need context and are not converted; nor are the per-entry
  notes.
