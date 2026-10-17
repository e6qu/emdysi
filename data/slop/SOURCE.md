# bheijden/slop rule sets

`rules/`: the rule sets of slop, a linter for machine-written prose:
`ai-tells.json`, `load-bearing.json`, `pr-vocabulary.json`,
`simonwillison.json` and `wikipedia-ai.json`. Each rule has a pattern, a
threshold, a description, a suggestion and tests (`hit`, `miss`).

- Upstream: https://github.com/bheijden/slop, commit
  6af32460028c96e5144c08fff2e2d44a2bbed9be (`rules/`, `LICENSE`, `NOTICE`,
  `README.md` as `README-upstream.md`).
- License: Apache License 2.0 (`LICENSE`), Copyright 2026 Bas van der
  Heijden, with the `NOTICE` kept. `simonwillison.json` and
  `wikipedia-ai.json` derive from Simon Willison's llm-cliche-highlighter
  (Apache-2.0). `wikipedia-ai.json` is adapted from Wikipedia's "Signs of
  AI writing", under CC BY-SA 4.0 (`LICENSE-CC-BY-SA-4.0.txt`, the text
  from the UD English EWT distribution), which combines with emdysi's
  AGPL-3.0-or-later (decision D15).
- Unchanged.
- Use: `scripts/import-rules.py` converts the regex rules that fire on any
  match, and that pass their own tests, into `packs/slop.toml` (opt-in).
