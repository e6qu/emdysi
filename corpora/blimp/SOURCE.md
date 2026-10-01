# BLiMP: The Benchmark of Linguistic Minimal Pairs (sample)

- Upstream: https://github.com/alexwarstadt/blimp, `data/*.jsonl`, commit
  `3e56b06fcabca9b30822fc66435fca6b1aa40bb1` (2022-12-13).
- License: Creative Commons Attribution 4.0 International (CC BY 4.0). The
  upstream README states: "BLiMP is distributed under a
  [CC-BY](https://creativecommons.org/licenses/by/4.0/) license." The
  repository has no separate LICENSE file; `LICENSE` here is the license
  text from the SPDX License List
  (https://github.com/spdx/license-list-data, `text/CC-BY-4.0.txt`, commit
  `31ba1a50e539`).
- Copyright and attribution: Alex Warstadt, Alicia Parrish, Haokun Liu,
  Anhad Mohananey, Wei Peng, Sheng-Fu Wang and Samuel R. Bowman. "BLiMP:
  The Benchmark of Linguistic Minimal Pairs for English." Transactions of
  the Association for Computational Linguistics 8 (2020): 377–392.
  https://doi.org/10.1162/tacl_a_00321
- Provenance: sentences generated automatically from templates and a
  hand-built vocabulary by the authors (not taken from third-party text).
- Modifications: a sample, not the full data. `sample.tsv` holds the first
  30 pairs (by `pairID`) of each of the 67 paradigms (2,010 pairs),
  converted from JSON Lines to TSV with `sample.py` (columns: paradigm
  `UID`, `field`, `linguistics_term`, `sentence_good`, `sentence_bad`; all
  other fields dropped). No sentence was changed.
- Use: tests and evaluation only (`crates/emdysi-check/examples/minimal_pairs.rs`).
  The full data (67,000 pairs) can be evaluated by pointing the example at
  an upstream checkout; it is not vendored.
