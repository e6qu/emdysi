# LanguageTool English rules

`en/`: LanguageTool's rules and word lists for English: `grammar.xml` and
`style.xml` (pattern rules), the variety files under `en-US`, `en-GB`,
`en-AU`, `en-CA`, `en-NZ`, and lists of replacements (`replace.txt`),
wordy and redundant phrases (`wordiness.txt`, `redundancies.txt`),
spelling variants to keep consistent (`coherency.txt`), confusable words
by context (`wrongWordInContext.txt`), contractions, diacritics, and the
words taking *a* or *an* against their spelling (`det_a.txt`, `det_an.txt`).

- Upstream: https://github.com/languagetool-org/languagetool, commit
  5b59afaf3a12489eb34f09b555cb31f1174926c7,
  `languagetool-language-modules/en/src/main/resources/org/languagetool/rules/en/`
  (the shell script `indent.sh` left out); `COPYING.txt` from the
  repository root.
- License: GNU Lesser General Public License 2.1 or later (the files'
  headers: "either version 2.1 of the License, or (at your option) any
  later version"; `COPYING.txt`). Copyright (C) 2001-2021 Daniel Naber,
  Marcin Miłkowski and the LanguageTool contributors. Portions of the
  rules are converted from the After the Deadline grammar checker, under
  the GNU GPL (version unstated, so any version may be chosen; used under
  GPL-3.0-or-later, `LICENSE-GPL-3.0` from the SPDX License List). Both
  combine with emdysi's AGPL-3.0-or-later (decision D15).
- Unchanged.
- Use: rules that need only words, not LanguageTool's part-of-speech
  tagger, converted into emdysi packs by `scripts/import-languagetool.py`,
  and its word lists (the articles exceptions, variant spellings).
