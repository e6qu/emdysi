# proselint checks

`checks/`: proselint's checks, Python modules that define word lists,
phrase lists and patterns (clichés, redundancy, jargon, malapropisms,
skunked terms, and more), with the plain-text lists they read
(`cliches/diction`, `cliches/write-good`, `cliches/garner`,
`redundancy/garner`, `redundancy/after-the-deadline`, `restricted/*`,
`needless-variants`). `tests/examples.py`: the sentences proselint's
tests expect each module to flag or pass. `engine.py`: the definitions of
the patterns' padding.

- Upstream: https://github.com/amperser/proselint, commit
  dbed789caae662d06c7c8a5a13dd31f1acd36f5c (`proselint/checks/`,
  `proselint/registry/checks/engine.py`, `tests/examples.py`,
  `LICENSE.md`).
- License: BSD 3-Clause (`LICENSE.md`), Copyright (c) 2014-2015 Jordan
  Suchow, Michael Pacer and Lara A. Ross. Lists drawn from other
  projects, as the modules' headers say: After the Deadline (GPL),
  GNU diction (GPL), write-good (MIT); usage lists cite Garner's Modern
  American Usage and other guides as sources of the advice. The GPL
  portions combine with emdysi's AGPL-3.0-or-later (decision D15).
- Unchanged.
- Use: `scripts/import-rules.py` reads the check modules with Python's
  `ast` module, without running them, and converts four kinds of check
  (existence of patterns or of one pattern, preferred forms of words or
  of patterns) into `packs/proselint.toml` (opt-in).
