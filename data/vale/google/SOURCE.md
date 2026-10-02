# Vale package for the Google developer documentation style guide

- Upstream: https://github.com/errata-ai/Google, commit
  `9e2483e2f09e7f623bacbbc154adb9627bed563b` (2026-09-28); retrieved
  2026-10-02. `Google/` and `LICENSE` are copied verbatim.
- License: MIT, "Copyright (c) 2018 - 2019 Joseph Kato" (`LICENSE`).
- The rules implement the Google developer documentation style guide
  (https://developers.google.com/style/), which the upstream README states
  is licensed CC BY 4.0. That statement could not be checked at the
  primary source from this environment (developers.google.com was not
  reachable); the vendored files themselves are the package's MIT-licensed
  rule files. Attribution: Google developer documentation style guide,
  Google LLC. The package is "neither maintained nor endorsed by Google"
  (upstream README).
- Provenance checked: no notice of other sources (After the Deadline,
  Wikipedia, LanguageTool) was found in the rule files.
- Use: `scripts/import-rules.py` converts the `existence` and
  `substitution` rules into `packs/google.toml` (opt-in pack `google`).
