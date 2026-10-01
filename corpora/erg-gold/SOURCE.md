# ERG gold test suites

- Upstream: https://github.com/delph-in/erg, `tsdb/gold/{mrs,csli,esd,control,ccs,sh-spec}`
- Commit: `13b615ab607374ed7cafe753830e23043a22b71d`
- License: MIT (ERG repository license, copied to `LICENSE` here).
- Provenance of the text:
  - `mrs` (107 items), `csli` (1,348 grammatical and ungrammatical items),
    `esd` (62, ERG semantic documentation), `control` (1,832) and `ccs`
    (63): test suites constructed by the grammar developers (item authors
    `oe` and `danf`), not excerpts of third-party text.
  - `sh-spec` (599): Arthur Conan Doyle, "The Adventure of the Speckled
    Band" (1892). Public domain (the author died in 1930).
  - Profiles built from third-party text whose copyright is not cleared
    (Wall Street Journal, Brown corpus, tourism brochures, e-commerce
    mail, TREC, FraCaS, Wikipedia, CGEL examples) are deliberately not
    vendored.
- The annotations (derivations, MRSs) are part of the ERG repository and
  covered by its MIT license.
- Format: [incr tsdb()] profiles. `item.gz` holds the sentences
  (`i-input` is field 7, `i-wf` field 11: 1 = grammatical, 0 = not);
  `result.gz` holds the gold derivation trees.
