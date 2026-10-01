# Test corpora

Third-party text used **only for tests and evaluation**. Each subdirectory
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

| Corpus | License | Contents |
|---|---|---|
| [`erg-gold/`](erg-gold/SOURCE.md) | MIT (ERG annotations); text by the ERG authors or public domain | ERG test suites `mrs`, `csli`, `esd`, `control`, `ccs` and the public-domain `sh-spec` (Sherlock Holmes) with gold derivations |
| [`golden-rules/`](golden-rules/SOURCE.md) | MIT | Sentence-segmentation test cases from pySBD / Pragmatic Segmenter |
