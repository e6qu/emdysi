# AI-style prose samples

Short documents written for this project in the style that the rule packs
target (machine-written prose: stock phrases, hedging, vague attribution,
signposting, chat-assistant residue), plus `control.md`, plainly written
prose that should draw few or no diagnostics.

- License: written for emdysi and covered by the repository's MIT license.
  No text was copied from other sources; names and figures are invented.
- Use: `crates/emdysi-check/tests/corpus.rs` checks each `*.md` file with
  the built-in packs and compares the diagnostics with `expected/<name>.txt`.
  After an intended change, regenerate the expected files with
  `UPDATE_EXPECTED=1 cargo test -p emdysi-check --test corpus` and review
  the diff.
