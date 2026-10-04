# Edited news and encyclopedia text, share-alike (sample)

**License: CC BY-SA 3.0** (`LICENSE-UD_English-PUD.txt`). Kept in its own
directory as `corpora/README.md` requires; used only for evaluation, never
to train shipped data.

- Upstream: https://github.com/UniversalDependencies/UD_English-PUD, commit
  `f16eba4ae7f3d161870ed320676c5088b8fa476c`, file
  `en_pud-ud-test.conllu`; retrieved 2026-10-04.
- License, from the upstream README ("Licenses and terms-of-use"): the
  sentences are "randomly selected Wikipedia (www.wikipedia.org)
  sentences", and Google makes them available "under CC-BY-SA 3.0"; the
  annotations are not used here.
- Copyright: the Wikipedia contributors (text); treebank by Google and the
  contributors listed in the upstream README (Uszkoreit, Macketanz,
  Burchardt, Harris, Marheinecke, Petrov, and others).
- Samples: `pud.tsv` (development) and `heldout-pud.tsv` (held-out test
  set). Selection (`../edited/sample.py`): the `# text` lines of the first 500
  sentences, and of the other 500,, in upstream order, grouped into documents by `# newdoc`; no
  text changed.
