# ERG grammar-error codes and feedback

- Upstream: https://github.com/delph-in/erg, `educ/ParserErrorCodes.xlsx`,
  commit `13b615ab607374ed7cafe753830e23043a22b71d` (the same commit as the
  vendored grammar; the spreadsheet itself is vendored at
  `grammar/erg/educ/ParserErrorCodes.xlsx`).
- License: MIT (the ERG's license, `grammar/erg/LICENSE`;
  `grammar/erg/educ/METADATA` declares `LICENSE="MIT"`).
- Provenance: feedback texts and example sentences written by the ERG
  developers for the grammar-error ("mal-rule") deployment of the ERG in
  writing instruction.
- `errors.tsv` is generated: `python3 gen-errors.py
  ../../grammar/erg/educ/ParserErrorCodes.xlsx > errors.tsv`. One row per
  error code (an ERG rule, lexical entry, lexical type or root): the code,
  the error class (`R`, `I`, `D`, `W`), the English feedback text (`$X`
  stands for the word) and the example sentences, separated by ` | `. The
  Chinese feedback texts and the comments columns are left out.
- Use: `core.grammar-errors` (`crates/emdysi-check`), for the messages of
  errors the grammar-error variant of the ERG finds.
