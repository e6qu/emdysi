# Vendored: English Resource Grammar (ERG)

- Upstream: https://github.com/delph-in/erg
- Commit: `13b615ab607374ed7cafe753830e23043a22b71d` (2026-05-22)
- License: MIT (see `LICENSE` in this directory; `METADATA` declares
  `LICENSE="MIT"`). File headers credit Dan Flickinger, Rob Malouf,
  Emily M. Bender and Stephan Oepen, "see LICENSE for conditions".
- License check: every vendored file was grepped for GPL/LGPL/Creative
  Commons/"all rights reserved" notices; none found (the only hits are
  lexicon entries for the word "GPL").

## What is included

Only the files needed by the default parsing configuration (`ace/config.tdl`), plus `ace/ace-erg-qc.txt` (quick-check paths):
`english.tdl` and everything it includes, `mtr.tdl`, `irregs.tab`, the REPP
tokenizer (`rpp/`), token-mapping rules (`tmr/`), variable-property mappings
(`*.vpm`), semantic interface files (`etc/*.smi`), `trigger.mtr`,
`idioms.mtr`, and the metadata/citation files.

The grammar-error ("mal-rule") variant used for naming errors, from the
same commit: `mal.tdl` (top file), `ace/config-mal.tdl`, and from `educ/`
the files `mal.tdl` includes (`tmr-educ`, `lfr-educ`, `ctype-educ`,
`lextypes-educ`, `letypes-mal`, `lexicon-mal`, `ple-educ`, `gle-educ`,
`constr-mal`, `lexrinst-mal`, `lexrinst-tok`, `roots-educ`), the irregular
forms it uses (`irregs-educ.tab`, `irregs-robust.tab`), its version and
metadata files (`Version-mal.lsp`, `METADATA`, `citation.bib`), and
`ParserErrorCodes.xlsx`, the table of error codes and feedback texts
(converted to `data/erg-errors/errors.tsv`). `educ/METADATA` declares
`LICENSE="MIT"`; the files carry the same copyright header as the rest of
the grammar.

## What is deliberately excluded

- `tsdb/` treebanks: several profiles contain third-party text (e.g. Wall
  Street Journal) whose copyright is not covered by the ERG license.
  Two ERG-authored test suites are vendored separately under
  `corpora/erg-gold/`.
- `redwoods.mem` (parse-ranking model, Git LFS) and the TnT POS-tagger models
  (`ace/english-postagger.hmm`, `agree/english-pos.hmm`): statistical models
  trained on corpora that include LDC-licensed text. Not used; v1 uses a
  rule-based tagger and heuristic ranking.
- WordNet-derived lexicon variants (`lex-wn.tdl`, `letypes-wn-*.tdl`, ...),
  dialect variants, the rest of `educ/` (paraphrase and masking variants,
  generation settings), tool-specific directories (`lkb/`, `pet/`,
  `agree/`, `openproof/`, `docs/`).

Do not edit files here; local changes belong in separate overlay files so the
vendored copy can be refreshed from upstream.
