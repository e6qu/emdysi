# Universal Dependencies English Web Treebank (EWT)

`en_ewt-ud-{train,dev,test}.conllu`: 16,622 sentences of English web text
(weblogs, newsgroups, emails, reviews, Yahoo! answers) with hand-checked
lemmas, parts of speech (universal and Penn Treebank), morphological
features and dependency trees, in CoNLL-U.

- Upstream: https://github.com/UniversalDependencies/UD_English-EWT,
  commit 4a4d77f599ea53cc405f85d0cec4b2f14f81d42b; `README.md` is kept as
  `README-upstream.md`.
- License: Creative Commons Attribution-ShareAlike 4.0 International
  (`LICENSE.txt`, copied unchanged). The text is the English Web Treebank
  (LDC2012T13), released by Google under CC BY-SA 4.0 through Universal
  Dependencies; see the upstream README for the contributors to cite.
- Unchanged.
- Use (test): measuring the part-of-speech tagger that feeds the grammar
  (decision D3) against hand-checked tags, and a sample of real web
  English for coverage.
