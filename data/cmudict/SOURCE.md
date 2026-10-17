# CMU Pronouncing Dictionary

`cmudict.dict`: about 135,000 English words with their pronunciations in
ARPAbet phonemes (stress marked on vowels: 0, 1, 2), one pronunciation a
line, variants as `word(2)`.

- Upstream: https://github.com/cmusphinx/cmudict, commit
  74790861f652b15e4ac49015a90074ad62a27690 (2025-10-24), files
  `cmudict.dict`, `LICENSE` and `README` (here `README-upstream`).
- License: BSD 2-Clause style, "Copyright (C) 1993-2015 Carnegie Mellon
  University" (`LICENSE`, copied unchanged). The license deems the
  dictionary source code; redistribution keeps the notice.
- Unchanged.
- Use: the indefinite article rule (`core.articles`) reads a word's first
  sound from it, where the spelling leaves it open (*an hour*, *a
  unicorn*, *a one-time*, *an x-ray*).
