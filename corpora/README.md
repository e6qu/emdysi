# Test corpora

Text used **only for tests and evaluation**: third-party corpora, and
samples written for this project. Each subdirectory
carries its own license and provenance note. Nothing here is compiled into the
published crates.

Rules for adding a corpus:

1. The license must permit redistribution (public domain, MIT/BSD/Apache, CC0,
   CC BY). CC BY-SA material may be added only in its own subdirectory with the
   license text, and must never be used to train shipped data.
   NonCommercial (NC) or no-derivatives (ND) material, LDC data and text of
   unclear copyright are not allowed.
2. Record the upstream URL, version/commit, license and a short provenance
   note in `<corpus>/SOURCE.md`, and copy the license text alongside.
3. Verify the license from the primary source, not a secondary listing.
4. Add an entry to [`VENDORED.toml`](../VENDORED.toml) (upstream, exact
   version, retrieval date, SPDX license, license file, copyright holders,
   provenance, modifications, use) and run
   `python3 scripts/check-vendored.py --update` to pin the files' hashes in
   `VENDORED.sha256`. CI fails on any file that is not registered or whose
   content changed.

| Corpus | License | Contents |
|---|---|---|
| [`erg-gold/`](erg-gold/SOURCE.md) | MIT (ERG annotations); text by the ERG authors or public domain | ERG test suites `mrs`, `csli`, `esd`, `control`, `ccs` and the public-domain `sh-spec` (Sherlock Holmes) with gold derivations |
| [`golden-rules/`](golden-rules/SOURCE.md) | MIT | Sentence-segmentation test cases from pySBD / Pragmatic Segmenter |
| [`ai-prose/`](ai-prose/README.md) | MIT (written for this project) | Short documents in the style of machine-written prose, plus a plainly written control, with the expected diagnostics of the built-in packs |
