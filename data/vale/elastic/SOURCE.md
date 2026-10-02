# Vale package for the Elastic style guide

- Upstream: https://github.com/elastic/vale-rules, commit
  `edbc6fab94cf62728753d6cc55c30109b8aee65a` (2026-10-01); retrieved
  2026-10-02. `styles/Elastic/*.yml` (as `Elastic/`), `LICENSE` and
  `NOTICE.txt` are copied verbatim.
- License: Apache-2.0 (`LICENSE`); `NOTICE.txt`: "Elastic style guide for
  the Vale linter / Copyright 2025 Elasticsearch B.V." The NOTICE file is
  kept, as section 4(d) of the license requires for redistribution.
- Provenance checked: the README says the rules are "based on the Elastic
  style guide and recommendations"; no notice of other sources was found.
- Use: `scripts/import-rules.py` converts the `existence` and
  `substitution` rules into `packs/elastic.toml` (opt-in pack `elastic`).
