# Open English WordNet: words, parts of speech and verb frames

`words.tsv` lists the single-word, lower-case lemmas of Open English
WordNet with their parts of speech (n, v, a, r), the verb frames of their
senses (named in the upstream `src/yaml/frames.yaml`: `vtai` "Somebody
----s something", `via` "Somebody ----s", ...) and their irregular forms.

- Upstream: https://github.com/globalwordnet/english-wordnet, commit
  bff3181fe5c810dcd157cba0eed60322a6e0aaed (2026-08-26), `src/yaml/entries-*.yaml`.
- Made by `scripts/oewn/extract.py` (`extract.py REPO/src/yaml >
  data/oewn/words.tsv`): multiword lemmas, names (capitalized lemmas),
  senses, definitions, examples and relations are left out.
- License: Creative Commons Attribution 4.0 for Open English WordNet, over
  the Princeton WordNet 3.1 license for the data it derives from
  (`LICENSE`, `LICENSE-WordNet`, both copied from the upstream
  repository). Both permit use, modification and redistribution with
  attribution: Open English WordNet, Copyright (c) 2019-present, The Open
  English Wordnet Team; WordNet 3.1, Copyright 2011 by Princeton University.
- Use: evidence for the grammar's lexical gaps (a word the ERG lacks in a
  part of speech or a frame; see docs/evaluation.md).
