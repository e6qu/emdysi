# Vale package for the Microsoft Writing Style Guide

- Upstream: https://github.com/errata-ai/Microsoft, commit
  `ea5fa73c530a8413f39701b01856258c0223f101` (2026-09-28); retrieved
  2026-10-02. `Microsoft/` and `LICENSE` are copied verbatim.
- License: MIT, "Copyright (c) 2018 - 2019 Joseph Kato" (`LICENSE`).
- The rules implement the Microsoft Writing Style Guide, which Microsoft
  publishes under CC BY 4.0 (`LICENSE-style-guide`, copied verbatim from
  https://github.com/MicrosoftDocs/microsoft-style-guide at commit
  `c6945c32294e845a84b192a094fb1b7c2c452a6a`). Attribution: Microsoft
  Writing Style Guide, Microsoft Corporation,
  https://learn.microsoft.com/en-us/style-guide/welcome/. The package is
  "neither maintained nor endorsed by Microsoft" (upstream README).
- Provenance checked: the rule files contain word lists and patterns
  written by the package authors from the style guide; no notice of other
  sources (After the Deadline, Wikipedia, LanguageTool) was found.
- Use: `scripts/import-rules.py` converts the `existence` and
  `substitution` rules into `packs/microsoft.toml` (opt-in pack
  `microsoft`), keeping each rule's link as its `source`. Other rule kinds
  are not converted.
