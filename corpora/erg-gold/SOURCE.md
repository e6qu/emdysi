# ERG gold test suites

- Upstream: https://github.com/delph-in/erg, `tsdb/gold/{mrs,csli}`
- Commit: `13b615ab607374ed7cafe753830e23043a22b71d`
- License: MIT (ERG repository license, copied to `LICENSE` here).
- Provenance: both are test suites constructed by the grammar developers
  (the "MRS" test suite of 107 sentences and the CSLI test suite of 1,348
  grammatical and ungrammatical items), not excerpts of third-party text.
- Format: [incr tsdb()] profiles. `item.gz` holds the sentences
  (`i-input` is field 7, `i-wf` field 11: 1 = grammatical, 0 = not);
  `result.gz` holds the gold derivation trees.
