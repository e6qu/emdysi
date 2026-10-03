# AI-prose treebank

Sentences of machine-written and machine-style prose with the reading of
the English Resource Grammar that is right, chosen by hand: the reference
for how well emdysi's parse ranker does on the kind of text it checks, and
extra training data for it.

- `treebank.tsv`: one sentence per line: an id, the source, the sentence,
  and the skeleton of the right derivation (rule and lexical entry names
  with chart positions, as `crates/emdysi-parse/examples/train.rs` matches
  them), or `-` when none of the six best readings was right; a note says
  what the best reading got wrong, when it did.
- How it was made: `crates/emdysi-parse/examples/judge.rs` printed, for each
  sentence, the tree of the best reading and how each of the next five
  differs from it (phrases grouped differently, words with a different
  lexical entry). The right reading was chosen by judging the English, not
  by agreement with another parser. When no reading was entirely right,
  the sentence is marked `-`, except where a note says `partial`.
- Sentences: the 36 sentences of `corpora/ai-prose` between 6 and 28 words
  (except `control.md`), and 124 sentences sampled from the model outputs
  of `corpora/beemo`, with the same length limit and without inline code,
  parentheses or brackets.
- Result (2026-10-03, the ranker as of then): of 160 sentences, 97 have a
  right reading among the six best; parsed in their documents
  (`crates/emdysi-check/examples/treebank_eval.rs`), the best reading is
  the right one for 57 of them, 58 when the rest of the document settles
  close calls (see `docs/evaluation.md`). The other 63 are mostly
  fragments (no full analysis) and long narrative sentences from Beemo.

License: the annotations were made for this project and are covered by the
repository's MIT license. The `ai-prose` sentences are also MIT (this
repository). The Beemo sentences are model outputs distributed by Toloka
under MIT, subject to the generating models' terms (zephyr-7b-beta: MIT;
Mistral-7B-Instruct-v0.1 and Mixtral-8x7B-Instruct-v0.1: Apache-2.0); see
`corpora/beemo/SOURCE.md` and `corpora/beemo/LICENSE`.
